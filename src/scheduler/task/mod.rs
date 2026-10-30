//! [`Task`], its constructors, and [`Stepper`], which runs one step by step. The async driver,
//! [`drive()`](super::drive()), is a sibling of this module: it runs a task through `Stepper`
//! like any other scheduler.
//!
//! This module holds the task itself, and every submodule one constructor with the state type it
//! names.

mod future;
mod once;
mod periodic;
#[cfg(feature = "futures")]
mod stream;
#[cfg(feature = "futures")]
mod stream_acked;
mod wakeable;

pub use future::FutureThenContext;
pub use once::OnceContext;
pub use periodic::PeriodicContext;
#[cfg(feature = "futures")]
pub use stream::StreamThenContext;
#[cfg(feature = "futures")]
pub use stream_acked::{StreamAck, StreamAckedThenContext};
pub use wakeable::{TaskWaker, WakeableContext};

use educe::Educe;
use std::{pin::Pin, task::Context, time::Instant};

/// What a scheduler runs: a plain state `TC`, a state `P` that must be pinned (usually a future,
/// `()` when there is none), and a handler that drives them.
///
/// The handler is a `fn` pointer rather than a closure so that the task's type can be named in a
/// `where` clause (`S: Scheduler<TC, P>`). A `fn` pointer is always `Send`, so `Task<TC, P>: Send`
/// exactly when `TC: Send` and `P: Send`. Each constructor names its state type the same
/// way (`OnceContext`, `PeriodicContext`, …).
///
/// A task runs either through [`drive()`](super::drive()), on an async executor, or step by step
/// through [`split`](Self::split) and [`Stepper::step`], from a synchronous loop (a UI event loop,
/// a game frame loop).
#[derive(Educe)]
#[educe(Debug)]
pub struct Task<TC, P = ()> {
    #[educe(Debug(ignore))]
    context: TC,
    #[educe(Debug(ignore))]
    pinned: P,
    handler: TaskHandler<TC, P>,
}

/// The handler of a [`Task`]: it does some work, and answers when it wants to be called again.
/// [`Next::Pending`] and [`Next::PendingUntil`] mean the waker in `cx` was registered,
/// as a `Pending` of `Future::poll` does; the other states ignore it.
///
/// The [`Instant`] is the time of the step on the scheduler's clock
/// ([`SchedulerTypes::clock`](crate::scheduler::SchedulerTypes::clock)), the clock a
/// [`Next::SleepUntil`] is measured on. A handler takes the time from there rather than from
/// `Instant::now()`, so that it follows a scheduler whose clock is not the system's.
pub type TaskHandler<TC, P> = fn(&mut TC, Pin<&mut P>, &mut Context<'_>, Instant) -> Next;

/// What a handler asks the scheduler to do next. `Sleep*` waits for the clock only, `Pending*`
/// for the waker the handler registered, and `*Until` has a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
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
    /// Call the handler again when the waker in `cx` is woken. The handler registered the waker,
    /// which may be woken before the handler returns.
    Pending,
    /// Call the handler again when the waker in `cx` is woken, or at this instant, whichever comes
    /// first: a [`Pending`](Self::Pending) with a deadline. The handler must accept being called
    /// before the instant, since any wake of the task, a spurious one included, ends the wait. An
    /// instant that has already passed is a [`Yield`](Self::Yield).
    ///
    /// This is what a task waiting for events and for a timer at once returns.
    PendingUntil(Instant),
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
    /// use rx_rust::scheduler::{Task, Next};
    /// use std::{pin::pin, task::{Context, Waker}, time::Instant};
    ///
    /// let task = Task::new(0, |count, _, _, _| {
    ///     *count += 1;
    ///     if *count < 3 { Next::Yield } else { Next::Finished }
    /// });
    /// let (mut stepper, pinned) = task.split();
    /// let mut pinned = pin!(pinned);
    /// let mut cx = Context::from_waker(Waker::noop());
    /// let mut steps = 0;
    /// while stepper.step(pinned.as_mut(), &mut cx, Instant::now()) != Next::Finished {
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
/// - [`Finished`](Next::Finished): drop the task;
/// - [`Yield`](Next::Yield): let other tasks run, then step again;
/// - [`SleepUntil(at)`](Next::SleepUntil): step again at `at` at the earliest, on the
///   scheduler's clock, and in any case not before other tasks had a turn;
/// - [`Pending`](Next::Pending): the handler registered the waker in `cx`; do not step again
///   before it is woken. It may be woken before `step` returns, which the caller must handle (with
///   a flag, say);
/// - [`PendingUntil(at)`](Next::PendingUntil): step again once the waker in `cx` is woken, or
///   at `at`, whichever comes first, and in any case not before other tasks had a turn. As for
///   `Pending`, the waker may be woken before `step` returns.
///
/// The `delay` of [`run_task`](crate::scheduler::Scheduler::run_task) is the caller's to
/// implement: wait that long before the first `step`.
#[derive(Educe)]
#[educe(Debug)]
pub struct Stepper<TC, P> {
    #[educe(Debug(ignore))]
    context: TC,
    handler: TaskHandler<TC, P>,
}

impl<TC, P> Stepper<TC, P> {
    /// Drives one step: calls the handler once and returns its answer. `pinned` must be the state
    /// [`split`](Task::split) out of the same task, and `now` the scheduler's current time
    /// (that of [`SchedulerTypes::clock`](crate::scheduler::SchedulerTypes::clock)).
    pub fn step(&mut self, pinned: Pin<&mut P>, cx: &mut Context<'_>, now: Instant) -> Next {
        (self.handler)(&mut self.context, pinned, cx, now)
    }
}
