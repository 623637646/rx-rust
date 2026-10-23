use super::{Next, Task};
use crate::thread_mode::{ThreadMode, mutable::MutableHelper};
use educe::Educe;
use std::{task::Waker, time::Instant};

/// The state of [`Task::wakeable`].
///
/// Dropping it — the task finished, or was disposed — clears the waker it registered, so that a
/// [`TaskWaker`] that outlives the task neither keeps that waker alive nor wakes it.
pub struct WakeableContext<C, M: ThreadMode> {
    state: C,
    step: fn(&mut C, bool, Instant) -> Next,
    slot: M::Ptr<Slot>,
}

impl<C, M: ThreadMode> Drop for WakeableContext<C, M> {
    fn drop(&mut self) {
        drop(self.slot.with_mut(|slot| slot.waker.take()));
    }
}

/// The handle [`Task::wakeable`] returns, to wake the task from outside: from another thread than
/// the scheduler's when `M` is [`Shared`](crate::thread_mode::Shared), whatever the scheduler.
///
/// Dropping it wakes the task one last time, and tells it that nothing will wake it anymore.
#[derive(Educe)]
#[educe(Debug)]
pub struct TaskWaker<M: ThreadMode>(#[educe(Debug(ignore))] M::Ptr<Slot>);

/// What the task and its [`TaskWaker`] share, behind one lock: the waker the task registered
/// before its step.
struct Slot {
    waker: Option<Waker>,
    /// Set once the [`TaskWaker`] is dropped.
    closed: bool,
}

impl<C, M: ThreadMode> Task<WakeableContext<C, M>> {
    /// Calls `step(&mut state, closed, now)` — `now` the scheduler's time — when the task
    /// starts, then whenever the returned [`TaskWaker`] wakes it, until `step` returns
    /// [`Next::Finished`]. `closed` is whether the waker has been dropped: nothing will wake the
    /// task anymore, so a step left with nothing to wait for finishes.
    ///
    /// `step` answers as any handler does: [`Next::Pending`] waits for a wake,
    /// [`Next::PendingUntil`] for a wake or the instant, whichever comes first. This is a task that
    /// waits for events coming from elsewhere, which cannot schedule anything themselves — a
    /// producer on another thread than a single-threaded scheduler's, say.
    ///
    /// No wake is lost as long as the code that wakes the task changes what `step` looks at, then
    /// calls [`TaskWaker::wake`]: the task is ready to be woken before `step` is called, so a
    /// change that `step` missed is followed by a wake that steps it again. That holds whatever
    /// orders the change and the look — a lock, or atomics of any ordering — since the task
    /// registers before the look and the waker takes the waker after the change, both under one
    /// lock. A step may also come without a change — a spurious wake, a deadline, a wake for a
    /// change an earlier step already took — so `step` looks at its state rather than assume why it
    /// was called.
    ///
    /// `M` is the thread mode of the state the task and the waker share: [`Shared`] when the waker
    /// is used from another thread than the scheduler's, [`Local`] when both stay on one thread.
    ///
    /// [`Shared`]: crate::thread_mode::Shared
    /// [`Local`]: crate::thread_mode::Local
    ///
    /// ```
    /// use rx_rust::{
    ///     scheduler::{Next, Scheduler, Task, TaskWaker, virtual_time::VirtualTime},
    ///     thread_mode::{Shared, mutable::{MutableExt, MutableHelper}},
    /// };
    /// use std::{sync::{Arc, Mutex}, time::Duration};
    ///
    /// let time = VirtualTime::new();
    /// // What a producer sends, and what the task has taken of it.
    /// let inbox = Arc::new(Mutex::new(Vec::new()));
    /// let taken = Arc::new(Mutex::new(Vec::new()));
    /// let (task, waker): (_, TaskWaker<Shared>) = Task::wakeable(
    ///     (inbox.clone(), taken.clone()),
    ///     |(inbox, taken), closed, _| {
    ///         let values = inbox.take_value();
    ///         taken.with_mut(|taken| taken.extend(values));
    ///         if closed { Next::Finished } else { Next::Pending }
    ///     },
    /// );
    /// let _disposal = time.scheduler().run_task(task, None);
    ///
    /// // The producer, on any thread: changes the state, then wakes the task.
    /// inbox.with_mut(|inbox| inbox.push(1));
    /// waker.wake();
    /// time.advance_by(Duration::ZERO);
    /// assert_eq!(taken.clone_value(), [1]);
    ///
    /// // Dropping the waker: the task takes what is left, and finishes.
    /// inbox.with_mut(|inbox| inbox.push(2));
    /// drop(waker);
    /// time.advance_by(Duration::ZERO);
    /// assert_eq!(taken.clone_value(), [1, 2]);
    /// assert_eq!(time.pending_tasks(), 0);
    /// ```
    pub fn wakeable(state: C, step: fn(&mut C, bool, Instant) -> Next) -> (Self, TaskWaker<M>) {
        let slot = M::ptr(Slot {
            waker: None,
            closed: false,
        });
        let task = Self::new(
            WakeableContext {
                state,
                step,
                slot: slot.clone(),
            },
            |context, _, cx, now| {
                // Registered before the step looks at its state, so that a change it misses wakes
                // it again.
                let (closed, replaced) = context.slot.with_mut(|slot| {
                    let replaced = match &slot.waker {
                        Some(waker) if waker.will_wake(cx.waker()) => None,
                        _ => slot.waker.replace(cx.waker().clone()),
                    };
                    (slot.closed, replaced)
                });
                // Outside the lock: dropping a waker can run arbitrary code of its executor.
                drop(replaced);
                (context.step)(&mut context.state, closed, now)
            },
        );
        (task, TaskWaker(slot))
    }
}

impl<M: ThreadMode> TaskWaker<M> {
    /// Wakes the task, after a change its step must see. A task that is stepping steps once more;
    /// one already woken since its last step, or finished, is left as it is.
    pub fn wake(&self) {
        if let Some(waker) = self.0.with_mut(|slot| slot.waker.take()) {
            waker.wake();
        }
    }
}

impl<M: ThreadMode> Drop for TaskWaker<M> {
    fn drop(&mut self) {
        let waker = self.0.with_mut(|slot| {
            slot.closed = true;
            slot.waker.take()
        });
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
