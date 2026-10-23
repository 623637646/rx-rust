//! A scheduler on a virtual clock, for tests: [`VirtualTime`] owns the clock and the queued tasks,
//! and moves the clock forward only when told to; [`VirtualTimeScheduler`] is the handle the
//! operators are given.
//!
//! A test then asserts on exact boundaries and takes no real time:
//!
//! ```rust
//! use rx_rust::{
//!     observable::ObservableExt, operators::creating::just::Just,
//!     scheduler::virtual_time::VirtualTime,
//! };
//! use std::{
//!     sync::{Arc, Mutex},
//!     time::Duration,
//! };
//!
//! let time = VirtualTime::new();
//! let values = Arc::new(Mutex::new(Vec::new()));
//! let values_observer = Arc::clone(&values);
//! let _subscription = Just::new(1)
//!     .delay(Duration::from_millis(100), time.scheduler())
//!     .subscribe_with_callback(move |value| values_observer.lock().unwrap().push(value), |_| {});
//!
//! time.advance_by(Duration::from_millis(99));
//! assert!(values.lock().unwrap().is_empty());
//! time.advance_by(Duration::from_millis(1));
//! assert_eq!(*values.lock().unwrap(), [1]);
//! ```
//!
//! # How tasks run
//!
//! [`run_task`](Scheduler::run_task) only queues a task, due at `now + delay`, even without a
//! delay. [`VirtualTime::advance_by`] then runs, on the calling thread, every task due up to the
//! new time, in the order they are due — those due at the same instant in the order they were
//! queued. Before running one, it sets the clock to the instant the task was due, so that a task
//! sees exactly the time it asked for: an `interval` advanced by ten periods at once emits ten
//! times, each at its own instant. A task queued while the clock advances runs in the same call if
//! it is due by then. [`advance_by(Duration::ZERO)`](VirtualTime::advance_by) runs the tasks due
//! now.
//!
//! A task that waits on something else than the clock ([`Next::Pending`]: a channel, a future of
//! another runtime) is queued again, at the current time, when it is woken: in the same
//! `advance_by` if something it runs wakes it, at the next one otherwise. A task that waits for
//! both ([`Next::PendingUntil`]) is queued at its instant, and again at the current time if it is
//! woken first. A task waiting on real IO or a real timer is therefore never woken while the clock
//! advances: such a pipeline belongs on a real scheduler.
//!
//! A delay too long for an [`Instant`] to represent never comes due: the task stays queued, and
//! counted by [`VirtualTime::pending_tasks`], until it is disposed.
//!
//! # Threads
//!
//! The scheduler is `Shared`: the clock belongs to the scheduler, not to a thread, so a test may
//! push events from other threads and still read one consistent time. The tasks run on the thread
//! that calls `advance_by`. Advancing from two threads at once, or from inside a task, panics.

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::{Shared, mutable::MutableHelper},
    utils::id_generator::{Id, IdGenerator},
};
use educe::Educe;
use std::{
    collections::{BTreeMap, HashMap},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Wake, Waker},
    time::{Duration, Instant},
};

/// The owner of a virtual clock and of the tasks queued on it. The test keeps it and moves the
/// clock with [`advance_by`](Self::advance_by); the operators get the
/// [`scheduler`](Self::scheduler), or the [`clock`](Self::clock) alone when they only read the
/// time.
///
/// The scheduler handles do not keep it alive: a task often holds a handle of its own scheduler
/// (an `interval` upstream of a `switch_map` whose inner observables are delayed on it, say), so a
/// strong handle would form a cycle through the queue. Dropping the `VirtualTime` drops every task
/// still queued, which cancels them, then those queued as they drop, until none is left: a task
/// whose drop always queues another keeps the drop from returning.
#[derive(Educe)]
#[educe(Debug)]
pub struct VirtualTime {
    #[educe(Debug(ignore))]
    state: Arc<Mutex<State>>,
}

/// The [`Scheduler`] of a [`VirtualTime`]: it queues the tasks there, and its
/// [`clock`](SchedulerTypes::clock) is the virtual clock.
///
/// # Panics
///
/// Running a task, or reading the time, panics once the [`VirtualTime`] has been dropped.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct VirtualTimeScheduler {
    #[educe(Debug(ignore))]
    state: Weak<Mutex<State>>,
}

/// The [`Clock`] of a [`VirtualTime`] and of its [`VirtualTimeScheduler`]: the virtual time,
/// readable from any thread.
///
/// # Panics
///
/// Reading the time panics once the [`VirtualTime`] has been dropped.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct VirtualClock {
    #[educe(Debug(ignore))]
    state: Weak<Mutex<State>>,
}

/// The handle of a task queued on a [`VirtualTime`]; disposing it drops the task, or, while the
/// task is running, drops it once its step returns.
#[derive(Debug)]
pub struct VirtualTimeDisposal {
    state: Weak<Mutex<State>>,
    id: Id,
}

/// How many steps in a row a task may ask to sleep until an instant that has already passed before
/// [`VirtualTime::advance_by`] gives up on it. On a real clock such a task waits for time to move;
/// on the virtual clock, which moves only between due tasks, it would loop forever. A heuristic,
/// to catch a deadline off by one in a test: a task that legitimately asks this many times in a row
/// is rejected too.
const MAX_STALLED_STEPS: u32 = 10_000;

/// A task as the queue holds it: its stepper and its pinned state, behind one call.
type Job = Box<dyn FnMut(&mut Context<'_>, Instant) -> Next + Send>;

/// Everything of a [`VirtualTime`], behind one lock. The lock never runs a task nor drops one: a
/// step and a drop can take it again (a disposal, a wake, a read of the clock), so they happen
/// outside.
struct State {
    now: Instant,
    ids: IdGenerator,
    next_sequence: u64,
    /// The waiting tasks due at some instant, by that instant, then by the order they were queued
    /// in. Every entry is the `key` of a [`WaitingTask`], removed with it.
    queue: BTreeMap<(Instant, u64), Id>,
    /// Every task but the running one.
    waiting: HashMap<Id, WaitingTask>,
    advancing: bool,
    /// The task whose step is running; its job is with `advance_by`. One at most, since
    /// `advance_by` does not run twice at once.
    running: Option<RunningTask>,
}

/// A task waiting for its turn. `key` is its entry in the queue, `None` when it is never due or
/// waits to be woken ([`Next::Pending`]); `wakeable`: a wake queues it again at the current time.
/// `stalled` counts its steps in a row that asked to sleep until an instant that had passed (see
/// [`MAX_STALLED_STEPS`]).
struct WaitingTask {
    job: Job,
    key: Option<(Instant, u64)>,
    wakeable: bool,
    stalled: u32,
}

/// The task whose step is running. A disposal meanwhile sets `cancelled`, a wake sets `woken`.
struct RunningTask {
    id: Id,
    cancelled: bool,
    woken: bool,
    stalled: u32,
}

/// What to do with a task after one of its steps.
enum AfterStep {
    Requeued,
    /// Finished or disposed: drop it, outside the lock.
    Done(Job),
    /// Asked [`MAX_STALLED_STEPS`] times in a row to sleep until an instant that had passed.
    Stalled(Job),
}

impl State {
    /// Queues `job`, due at `due`, or never when it is `None`; a wake queues it again at the
    /// current time when it is `wakeable`.
    fn push(&mut self, id: Id, job: Job, due: Option<Instant>, stalled: u32, wakeable: bool) {
        let key = due.map(|due| {
            let key = (due, self.next_sequence);
            self.next_sequence += 1;
            self.queue.insert(key, id);
            key
        });
        self.waiting.insert(
            id,
            WaitingTask {
                job,
                key,
                wakeable,
                stalled,
            },
        );
    }

    /// Takes a waiting task out, with its entry in the queue.
    fn unqueue(&mut self, id: Id) -> Option<WaitingTask> {
        let task = self.waiting.remove(&id)?;
        if let Some(key) = task.key {
            self.queue.remove(&key);
        }
        Some(task)
    }

    /// Takes out the next task due by `target` and sets the clock to when it was due; when none
    /// is left, sets the clock to `target`. Both under the one lock, so that a task queued from
    /// another thread meanwhile is either run by this `advance_by` or due after it.
    fn next_due(&mut self, target: Instant) -> Option<(Id, Job, Instant)> {
        let Some(entry) = self
            .queue
            .first_entry()
            .filter(|entry| entry.key().0 <= target)
        else {
            self.now = self.now.max(target);
            return None;
        };
        let ((due, _), id) = entry.remove_entry();
        let Some(WaitingTask { job, stalled, .. }) = self.waiting.remove(&id) else {
            unreachable!("every entry of the queue is the key of a waiting task");
        };
        self.running = Some(RunningTask {
            id,
            cancelled: false,
            woken: false,
            stalled,
        });
        self.now = self.now.max(due);
        Some((id, job, self.now))
    }

    /// Acts on the answer of a step of the running task.
    fn after_step(&mut self, job: Job, next: Next) -> AfterStep {
        let Some(RunningTask {
            id,
            cancelled: false,
            woken,
            stalled,
        }) = self.running.take()
        else {
            // Disposed while it ran.
            return AfterStep::Done(job);
        };
        match next {
            Next::Finished => AfterStep::Done(job),
            Next::Yield => {
                self.push(id, job, Some(self.now), 0, false);
                AfterStep::Requeued
            }
            // Woken before its step returned: it runs again rather than wait for a wake that
            // already came.
            Next::Pending | Next::PendingUntil(_) if woken => {
                self.push(id, job, Some(self.now), 0, false);
                AfterStep::Requeued
            }
            // Requeued with a new sequence even when the instant has passed, so that the other
            // tasks due now get their turn first.
            Next::SleepUntil(at) | Next::PendingUntil(at) => {
                let stalled = if at <= self.now { stalled + 1 } else { 0 };
                if stalled >= MAX_STALLED_STEPS {
                    return AfterStep::Stalled(job);
                }
                let wakeable = matches!(next, Next::PendingUntil(_));
                self.push(id, job, Some(at.max(self.now)), stalled, wakeable);
                AfterStep::Requeued
            }
            Next::Pending => {
                self.push(id, job, None, 0, true);
                AfterStep::Requeued
            }
        }
    }

    /// Queues a waiting task again, at the current time. A task already queued stays as it is, so
    /// that repeated wakes run it once, unless it waits for a wake, alone or with its instant: its
    /// entry, if any, is replaced by one now.
    fn wake(&mut self, id: Id) {
        if let Some(running) = &mut self.running
            && running.id == id
        {
            running.woken = true;
        } else if self.waiting.get(&id).is_some_and(|task| task.wakeable)
            && let Some(task) = self.unqueue(id)
        {
            self.push(id, task.job, Some(self.now), task.stalled, false);
        }
    }
}

impl Default for VirtualTime {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualTime {
    /// A virtual clock that starts at the current system time and moves only through
    /// [`advance_by`](Self::advance_by).
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                now: Instant::now(),
                ids: IdGenerator::default(),
                next_sequence: 0,
                queue: BTreeMap::new(),
                waiting: HashMap::new(),
                advancing: false,
                running: None,
            })),
        }
    }

    /// A scheduler that queues its tasks here and reads this clock.
    pub fn scheduler(&self) -> VirtualTimeScheduler {
        VirtualTimeScheduler {
            state: Arc::downgrade(&self.state),
        }
    }

    /// The virtual clock, for an operator that reads the time without scheduling anything
    /// (`throttle`, `timestamp`, `time_interval`): the [`clock`](SchedulerTypes::clock) of
    /// [`scheduler`](Self::scheduler).
    pub fn clock(&self) -> VirtualClock {
        VirtualClock {
            state: Arc::downgrade(&self.state),
        }
    }

    /// The current virtual time.
    pub fn now(&self) -> Instant {
        self.state.with_ref(|state| state.now)
    }

    /// The number of tasks queued, never due or waiting to be woken: what a disposal must bring
    /// down.
    pub fn pending_tasks(&self) -> usize {
        self.state
            .with_ref(|state| state.waiting.len() + usize::from(state.running.is_some()))
    }

    /// Moves the clock forward by `duration`, running on this thread every task due by then, each
    /// at the instant it was due (see the [module docs](self)). `Duration::ZERO` runs the tasks
    /// due now.
    ///
    /// A task that never stops yielding, or wakes itself in every step, makes this loop forever, as
    /// it would spin on a real executor.
    ///
    /// # Panics
    ///
    /// Panics when called from inside a task, or while another thread advances the same clock,
    /// and when the new time is too far for an [`Instant`] to represent; the clock is left as it
    /// was. A panic of a task is resumed here, after the task has been dropped.
    ///
    /// Also panics, after dropping the task, when a task keeps asking to sleep until an instant
    /// that has already passed: on a real clock it would wait for time to move, but this clock
    /// moves only between due tasks, so it would loop forever. A deadline compared with `<` where
    /// `<=` was meant does that.
    pub fn advance_by(&self, duration: Duration) {
        let target = self.state.with_mut(|state| {
            if state.advancing {
                return Err(
                    "VirtualTime::advance_by called while the clock is already advancing, from \
                     inside a task or from another thread",
                );
            }
            let target = state
                .now
                .checked_add(duration)
                .ok_or("VirtualTime::advance_by: the new time is too far to be represented")?;
            state.advancing = true;
            Ok(target)
        });
        let target = target.unwrap_or_else(|message| panic!("{message}"));
        let _guard = AdvancingGuard(&self.state);
        while let Some((id, mut job, now)) = self.state.with_mut(|state| state.next_due(target)) {
            let waker = Waker::from(Arc::new(QueueWaker {
                id,
                state: Arc::downgrade(&self.state),
            }));
            let step = job(&mut Context::from_waker(&waker), now);
            match self.state.with_mut(|state| state.after_step(job, step)) {
                AfterStep::Requeued => {}
                AfterStep::Done(job) => drop(job),
                AfterStep::Stalled(job) => {
                    drop(job);
                    panic!(
                        "a task asked {MAX_STALLED_STEPS} times in a row to sleep until an \
                         instant that had already passed: the virtual clock moves only between \
                         due tasks, so it would never stop. Is a deadline compared with `<` \
                         where `<=` was meant?"
                    );
                }
            }
        }
    }
}

impl Drop for VirtualTime {
    fn drop(&mut self) {
        // Dropping a task can dispose or queue others, which takes the lock: take them all out
        // first, and repeat until none is left.
        loop {
            let tasks = self.state.with_mut(|state| {
                state.queue.clear();
                std::mem::take(&mut state.waiting)
            });
            if tasks.is_empty() {
                break;
            }
            drop(tasks);
        }
    }
}

/// Clears the `advancing` flag when `advance_by` returns or unwinds. A task that panicked has no
/// job left, only its running record, which is cleared here.
struct AdvancingGuard<'a>(&'a Mutex<State>);

impl Drop for AdvancingGuard<'_> {
    fn drop(&mut self) {
        self.0.with_mut(|state| {
            state.advancing = false;
            state.running = None;
        });
    }
}

/// Wakes a task by queueing it again. Called from anywhere, a running task included: the lock is
/// never held while a task runs.
struct QueueWaker {
    id: Id,
    state: Weak<Mutex<State>>,
}

impl Wake for QueueWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if let Some(state) = self.state.upgrade() {
            state.with_mut(|state| state.wake(self.id));
        }
    }
}

impl VirtualTimeScheduler {
    fn state(&self) -> Arc<Mutex<State>> {
        self.state
            .upgrade()
            .expect("the VirtualTime of the VirtualTimeScheduler has been dropped")
    }
}

impl SchedulerTypes for VirtualTimeScheduler {
    type Mode = Shared;
    type Disposal = VirtualTimeDisposal;
    type Clock = VirtualClock;

    fn clock(&self) -> VirtualClock {
        VirtualClock {
            state: self.state.clone(),
        }
    }
}

impl Clock for VirtualClock {
    fn now(&self) -> Instant {
        self.state
            .upgrade()
            .expect("the VirtualTime of the VirtualClock has been dropped")
            .with_ref(|state| state.now)
    }
}

impl<TC, P> Scheduler<TC, P> for VirtualTimeScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(
        &self,
        task: Task<TC, P>,
        delay: Option<Duration>,
    ) -> DisposeOnDrop<Self::Disposal> {
        let (mut stepper, pinned) = task.split();
        let mut pinned: Pin<Box<P>> = Box::pin(pinned);
        let job: Job = Box::new(move |cx, now| stepper.step(pinned.as_mut(), cx, now));
        let id = self.state().with_mut(|state| {
            let id = state.ids.next_id();
            // Nothing in here can panic and drop the task under the lock: a delay too long to
            // represent is a task that never comes due.
            let due = state.now.checked_add(delay.unwrap_or_default());
            state.push(id, job, due, 0, false);
            id
        });
        DisposeOnDrop::new(VirtualTimeDisposal {
            state: self.state.clone(),
            id,
        })
    }
}

impl Disposable for VirtualTimeDisposal {
    fn dispose(self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let job = state.with_mut(|state| {
            if let Some(running) = &mut state.running
                && running.id == self.id
            {
                running.cancelled = true;
                return None;
            }
            state.unqueue(self.id).map(|task| task.job)
        });
        // Outside the lock: dropping the task can dispose others.
        drop(job);
    }
}
