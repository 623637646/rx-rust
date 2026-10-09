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
//! A task that waits on something else than the clock ([`Poll::Pending`]: a channel, a future of
//! another runtime) is queued again, at the current time, when it is woken: in the same
//! `advance_by` if something it runs wakes it, at the next one otherwise. A task waiting on real
//! IO or a real timer is therefore never woken while the clock advances: such a pipeline belongs
//! on a real scheduler.
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
    scheduler::{Scheduler, SchedulerTypes, Task, TaskState},
    thread_mode::{Shared, mutable::MutableHelper},
    utils::id_generator::{Id, IdGenerator},
};
use educe::Educe;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};

/// The owner of a virtual clock and of the tasks queued on it. The test keeps it and moves the
/// clock with [`advance_by`](Self::advance_by); the operators get the
/// [`scheduler`](Self::scheduler).
///
/// The scheduler handles do not keep it alive: a task often holds a handle of its own scheduler
/// (an `interval` upstream of a `debounce`, say), so a strong handle would form a cycle through
/// the queue. Dropping the `VirtualTime` drops every task still queued, which cancels them.
#[derive(Educe)]
#[educe(Debug)]
pub struct VirtualTime {
    #[educe(Debug(ignore))]
    state: Arc<Mutex<State>>,
}

/// The [`Scheduler`] of a [`VirtualTime`]: it queues the tasks there, and its
/// [`now`](SchedulerTypes::now) is the virtual clock.
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

/// The handle of a task queued on a [`VirtualTime`]; disposing it drops the task, or, while the
/// task is running, drops it once its step returns.
pub struct VirtualTimeDisposal {
    state: Weak<Mutex<State>>,
    id: Id,
}

/// How many steps in a row a task may ask to sleep until an instant that has already passed before
/// [`VirtualTime::advance_by`] gives up on it. On a real clock such a task waits for time to move;
/// on the virtual clock, which moves only between due tasks, it would loop forever.
const MAX_STALLED_STEPS: u32 = 10_000;

/// A task as the queue holds it: its stepper and its pinned state, behind one call.
type Job = Box<dyn FnMut(&mut Context<'_>, Instant) -> Poll<TaskState> + Send>;

/// Everything of a [`VirtualTime`], behind one lock. The lock never runs a task nor drops one: a
/// step and a drop can take it again (a disposal, a wake, a read of the clock), so they happen
/// outside.
struct State {
    now: Instant,
    ids: IdGenerator,
    next_sequence: u64,
    /// The queued tasks by when they are due, then by the order they were queued in. A task has at
    /// most one entry, none when it is never due; the entry of a disposed task stays until it
    /// comes up, and is skipped then.
    queue: BinaryHeap<Reverse<(Instant, u64, Id)>>,
    slots: HashMap<Id, Slot>,
    advancing: bool,
    /// The task whose step is running, so that a panic of it removes its slot.
    running: Option<Id>,
}

/// Where a task is. `stalled` counts its steps in a row that asked to sleep until an instant that
/// had passed (see [`MAX_STALLED_STEPS`]).
enum Slot {
    Queued {
        job: Job,
        stalled: u32,
    },
    /// Taken out to run a step. A disposal meanwhile sets `cancelled`, a wake sets `woken`.
    Running {
        cancelled: bool,
        woken: bool,
        stalled: u32,
    },
    /// Returned [`Poll::Pending`]: it waits to be woken.
    Parked(Job),
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
    /// Queues `job`, due at `due`, or never when it is `None`.
    fn push(&mut self, id: Id, job: Job, due: Option<Instant>, stalled: u32) {
        if let Some(due) = due {
            let sequence = self.next_sequence;
            self.next_sequence += 1;
            self.queue.push(Reverse((due, sequence, id)));
        }
        self.slots.insert(id, Slot::Queued { job, stalled });
    }

    /// Takes out the next task due by `target` and sets the clock to when it was due; when none
    /// is left, sets the clock to `target`. Both under the one lock, so that a task queued from
    /// another thread meanwhile is either run by this `advance_by` or due after it.
    fn next_due(&mut self, target: Instant) -> Option<(Id, Job, Instant)> {
        while let Some(&Reverse((due, _, id))) = self.queue.peek() {
            if due > target {
                break;
            }
            self.queue.pop();
            if let Some(slot) = self.slots.get_mut(&id)
                && let Slot::Queued { stalled, .. } = *slot
                && let Slot::Queued { job, .. } = std::mem::replace(
                    slot,
                    Slot::Running {
                        cancelled: false,
                        woken: false,
                        stalled,
                    },
                )
            {
                self.now = self.now.max(due);
                self.running = Some(id);
                return Some((id, job, self.now));
            }
        }
        self.now = self.now.max(target);
        None
    }

    /// Acts on the answer of a step.
    fn after_step(&mut self, id: Id, job: Job, poll: Poll<TaskState>) -> AfterStep {
        self.running = None;
        let (woken, stalled) = match self.slots.get(&id) {
            Some(&Slot::Running {
                cancelled: false,
                woken,
                stalled,
            }) => (woken, stalled),
            // Disposed while it ran.
            _ => {
                self.slots.remove(&id);
                return AfterStep::Done(job);
            }
        };
        match poll {
            Poll::Ready(TaskState::Finished) => {
                self.slots.remove(&id);
                AfterStep::Done(job)
            }
            Poll::Ready(TaskState::Yield) => {
                self.push(id, job, Some(self.now), 0);
                AfterStep::Requeued
            }
            // Requeued with a new sequence even when the instant has passed, so that the other
            // tasks due now get their turn first.
            Poll::Ready(TaskState::SleepUntil(at)) => {
                let stalled = if at <= self.now { stalled + 1 } else { 0 };
                if stalled >= MAX_STALLED_STEPS {
                    self.slots.remove(&id);
                    return AfterStep::Stalled(job);
                }
                self.push(id, job, Some(at.max(self.now)), stalled);
                AfterStep::Requeued
            }
            // Woken before its step returned: it runs again rather than wait for a wake that
            // already came.
            Poll::Pending if woken => {
                self.push(id, job, Some(self.now), 0);
                AfterStep::Requeued
            }
            Poll::Pending => {
                self.slots.insert(id, Slot::Parked(job));
                AfterStep::Requeued
            }
        }
    }

    /// Queues a waiting task again, at the current time. A task already queued stays as it is, so
    /// that repeated wakes run it once.
    fn wake(&mut self, id: Id) {
        match self.slots.get_mut(&id) {
            Some(Slot::Parked(_)) => {
                if let Some(Slot::Parked(job)) = self.slots.remove(&id) {
                    self.push(id, job, Some(self.now), 0);
                }
            }
            Some(Slot::Running { woken, .. }) => *woken = true,
            Some(Slot::Queued { .. }) | None => {}
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
                queue: BinaryHeap::new(),
                slots: HashMap::new(),
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

    /// The current virtual time.
    pub fn now(&self) -> Instant {
        self.state.with_ref(|state| state.now)
    }

    /// The number of tasks queued, never due or waiting to be woken: what a disposal must bring
    /// down.
    pub fn pending_tasks(&self) -> usize {
        self.state.with_ref(|state| state.slots.len())
    }

    /// Moves the clock forward by `duration`, running on this thread every task due by then, each
    /// at the instant it was due (see the [module docs](self)). `Duration::ZERO` runs the tasks
    /// due now.
    ///
    /// A task that never stops yielding makes this loop forever, as it would spin on a real
    /// executor.
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
            let waker = Waker::from(Arc::new(TaskWaker {
                id,
                state: Arc::downgrade(&self.state),
            }));
            let poll = job(&mut Context::from_waker(&waker), now);
            match self.state.with_mut(|state| state.after_step(id, job, poll)) {
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
            let slots = self.state.with_mut(|state| {
                state.queue.clear();
                std::mem::take(&mut state.slots)
            });
            if slots.is_empty() {
                break;
            }
            drop(slots);
        }
    }
}

/// Clears the `advancing` flag when `advance_by` returns or unwinds. A task that panicked has no
/// job left, only its `Running` slot, which is removed here.
struct AdvancingGuard<'a>(&'a Mutex<State>);

impl Drop for AdvancingGuard<'_> {
    fn drop(&mut self) {
        self.0.with_mut(|state| {
            state.advancing = false;
            if let Some(id) = state.running.take() {
                state.slots.remove(&id);
            }
        });
    }
}

/// Wakes a task by queueing it again. Called from anywhere, a running task included: the lock is
/// never held while a task runs.
struct TaskWaker {
    id: Id,
    state: Weak<Mutex<State>>,
}

impl Wake for TaskWaker {
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

    fn now(&self) -> Instant {
        self.state().with_ref(|state| state.now)
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
            state.push(id, job, due, 0);
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
        let job = state.with_mut(|state| match state.slots.get_mut(&self.id) {
            Some(Slot::Running { cancelled, .. }) => {
                *cancelled = true;
                None
            }
            Some(_) => match state.slots.remove(&self.id) {
                Some(Slot::Queued { job, .. } | Slot::Parked(job)) => Some(job),
                _ => None,
            },
            None => None,
        });
        // Outside the lock: dropping the task can dispose others.
        drop(job);
    }
}
