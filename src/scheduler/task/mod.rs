//! [`Task`], its constructors, and how to run one: [`Stepper`] step by step, [`drive()`] on an
//! async executor.
//!
//! This module holds the task itself; `drive` holds the async driver, and every other submodule
//! one constructor with the state type it names.

mod drive;
mod future;
mod once;
mod periodic;
mod recursive;
#[cfg(feature = "futures")]
mod stream;

pub use drive::{drive, yield_now};
pub use future::FutureThenContext;
pub use once::OnceContext;
pub use periodic::PeriodicContext;
pub use recursive::RecursiveContext;
#[cfg(feature = "futures")]
pub use stream::StreamThenContext;

use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

/// What a scheduler runs: a plain state `TC`, a state `P` that must be pinned (usually a future,
/// `()` when there is none), and a handler that drives them.
///
/// The handler is a `fn` pointer rather than a closure so that the task's type can be named in a
/// `where` clause (`S: Scheduler<TC, P>`). A `fn` pointer is always `Send`, so `Task<TC, P>: Send`
/// exactly when `TC: Send` and `P: Send`. Each constructor names its state type the same
/// way (`OnceContext`, `PeriodicContext`, …).
///
/// A task runs either through [`drive()`], on an async executor, or step by step through
/// [`split`](Self::split) and [`Stepper::step`], from a synchronous loop (a UI event loop, a game
/// frame loop).
pub struct Task<TC, P = ()> {
    context: TC,
    pinned: P,
    handler: TaskHandler<TC, P>,
}

/// The handler of a [`Task`]. It has the shape of `Future::poll`: [`Poll::Pending`] means the
/// waker in `cx` was registered and the handler waits to be woken; [`Poll::Ready`] tells the
/// scheduler what to do next. A handler that only forwards a poll is
/// `future.poll(cx).map(|_| TaskState::Finished)`.
///
/// The [`Instant`] is the time of the step on the scheduler's clock
/// ([`SchedulerTypes::now`](crate::scheduler::SchedulerTypes::now)), the clock a
/// [`TaskState::SleepUntil`] is measured on. A handler takes the time from there rather than from
/// `Instant::now()`, so that it follows a scheduler whose clock is not the system's.
pub type TaskHandler<TC, P> =
    fn(&mut TC, Pin<&mut P>, &mut Context<'_>, Instant) -> Poll<TaskState>;

/// What a handler that is ready asks the scheduler to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// The task is done; the handler is not called again.
    Finished,
    /// Give the executor a turn, then call the handler again.
    Yield,
    /// Call the handler again at this instant, or after a [`Yield`](Self::Yield) if it has
    /// already passed, so that there is always a yield point. The instant is on the scheduler's
    /// clock: the `now` the handler is given, plus some duration.
    ///
    /// The instant is absolute so that the time the handler spends after choosing it — delivering
    /// events, say — does not push the next call back: the scheduler measures the wait only when
    /// it starts waiting.
    SleepUntil(Instant),
}

impl<TC> Task<TC> {
    /// A task without a pinned state.
    pub fn new(context: TC, handler: TaskHandler<TC, ()>) -> Self {
        Self::with_pinned(context, (), handler)
    }
}

impl<TC, P> Task<TC, P> {
    /// A task with a state `pinned` that the handler gets pinned.
    pub fn with_pinned(context: TC, pinned: P, handler: TaskHandler<TC, P>) -> Self {
        Self {
            context,
            pinned,
            handler,
        }
    }

    /// Splits the task into its driver and its pinned state, for a scheduler that drives it step by
    /// step: the caller pins `P` (`std::pin::pin!` or `Box::pin`) and passes it to
    /// [`Stepper::step`]. Pinning the whole task instead would take `unsafe` or pin projection to
    /// reach `P`.
    ///
    /// ```
    /// use rx_rust::scheduler::{Task, TaskState};
    /// use std::{pin::pin, task::{Context, Poll, Waker}, time::Instant};
    ///
    /// let task = Task::new(0, |count, _, _, _| {
    ///     *count += 1;
    ///     Poll::Ready(if *count < 3 { TaskState::Yield } else { TaskState::Finished })
    /// });
    /// let (mut stepper, pinned) = task.split();
    /// let mut pinned = pin!(pinned);
    /// let mut cx = Context::from_waker(Waker::noop());
    /// let mut steps = 0;
    /// while stepper.step(pinned.as_mut(), &mut cx, Instant::now())
    ///     != Poll::Ready(TaskState::Finished)
    /// {
    ///     steps += 1;
    /// }
    /// assert_eq!(steps, 2);
    /// ```
    pub fn split(self) -> (Stepper<TC, P>, P) {
        (
            Stepper {
                context: self.context,
                handler: self.handler,
            },
            self.pinned,
        )
    }
}

/// The driver [`Task::split`] returns: everything of the task except its pinned state.
///
/// This is the lowest-level interface of a custom scheduler. Every [`step`](Self::step) calls the
/// handler once, with the scheduler's current time, and the caller acts on the answer:
/// - [`Poll::Pending`]: the handler registered the waker in `cx`; do not step again before it is
///   woken. It may be woken before `step` returns, which the caller must handle (with a flag, say);
/// - [`Finished`](TaskState::Finished): drop the task;
/// - [`Yield`](TaskState::Yield): let other tasks run, then step again;
/// - [`SleepUntil(at)`](TaskState::SleepUntil): step again at `at` at the earliest, on the
///   scheduler's clock, and in any case not before other tasks had a turn.
///
/// The `delay` of [`run_task`](crate::scheduler::Scheduler::run_task) is the caller's to
/// implement: wait that long before the first `step`.
pub struct Stepper<TC, P> {
    context: TC,
    handler: TaskHandler<TC, P>,
}

impl<TC, P> Stepper<TC, P> {
    /// Drives one step: calls the handler once and returns its answer. `pinned` must be the state
    /// [`split`](Task::split) out of the same task, and `now` the scheduler's current time
    /// ([`SchedulerTypes::now`](crate::scheduler::SchedulerTypes::now)).
    pub fn step(
        &mut self,
        pinned: Pin<&mut P>,
        cx: &mut Context<'_>,
        now: Instant,
    ) -> Poll<TaskState> {
        (self.handler)(&mut self.context, pinned, cx, now)
    }
}
