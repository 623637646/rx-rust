//! Running work later, or elsewhere: the [`Scheduler`] trait and its implementations.
//!
//! Every time-based operator (`delay`, `debounce`, `timeout`, `interval`, …) and every operator
//! that moves work between threads (`observe_on`, `subscribe_on`) takes a [`Scheduler`] value, so
//! nothing is global and one program can drive different pipelines on different runtimes. The
//! schedulers of [`runtime`] are selected by feature flags, which can be combined:
//!
//! | Feature               | Scheduler                                     | Mode     |
//! |-----------------------|-----------------------------------------------|----------|
//! | `tokio-scheduler`     | `runtime::tokio::TokioScheduler`              | `Shared` |
//! | `tokio-scheduler`     | `runtime::tokio::TokioLocalScheduler`         | `Local`  |
//! | `smol-scheduler`      | `runtime::smol::SmolScheduler`                | `Shared` |
//! | `smol-scheduler`      | `runtime::smol::SmolLocalScheduler`           | `Local`  |
//! | `futures-scheduler`   | `runtime::futures::ThreadPoolScheduler`       | `Shared` |
//! | `futures-scheduler`   | `runtime::futures::LocalPoolScheduler`        | `Local`  |
//! | (always)              | [`virtual_time::VirtualTimeScheduler`]        | `Shared` |
//!
//! [`VirtualTimeScheduler`](virtual_time::VirtualTimeScheduler) runs on a virtual clock that a
//! test moves forward, for exact and instant time-based tests.
//! # Executor lifetime
//!
//! A task can reach its own scheduler: `debounce` keeps its scheduler in its observer, and an
//! `interval` upstream on the same scheduler holds that observer in its task. An executor that runs
//! only while someone drives it — a `LocalSet`, a smol executor, a `LocalPool` — would then be kept
//! alive by its own pending tasks once nobody drives it, so the schedulers of those hold it weakly,
//! and dropping it cancels its tasks. A Tokio runtime is not kept alive by its handle either. A
//! [`ThreadPoolScheduler`](runtime::futures::ThreadPoolScheduler) holds its pool: the pool's
//! threads always run, so a disposed task is dropped and lets the pool go.
//!
//! # Tasks
//!
//! A scheduler runs a [`Task`]: a state, an optional pinned state (usually a future) and a `fn`
//! handler. Unlike a closure or an `async` block, such a task has a type an operator can name in a
//! `where` clause (`S: Scheduler<DelayTask<…>>`), which lets each scheduler state what it requires
//! of it: `Send` for a multi-threaded one, nothing for a single-threaded one. Code that holds a
//! concrete closure or future uses [`SchedulerExt`] instead.
//!
//! Implementing a scheduler takes [`SchedulerTypes`] and [`Scheduler::run_task`]. On an async
//! executor, spawn the future [`drive`] returns; a synchronous loop drives the task itself through
//! [`Task::split`] and [`Stepper::step`].
//!
//! # Time
//!
//! Time comes from the scheduler, never from `Instant::now()`: an operator reads
//! [`SchedulerTypes::now`] at subscription or on an event, and a task gets the time of each step
//! from the scheduler that runs it. A scheduler with a clock of its own therefore drives every
//! deadline and timestamp of the operators it is given.
//!
//! # Examples
//! ```rust
//! # #[cfg(not(feature = "tokio-scheduler"))]
//! # fn main() {}
//! # #[cfg(feature = "tokio-scheduler")]
//! #[tokio::main]
//! async fn main() {
//!     use rx_rust::scheduler::SchedulerExt;
//!     use std::{sync::{Arc, Mutex}, time::Duration};
//!
//!     use rx_rust::scheduler::runtime::tokio::TokioScheduler;
//!
//!     let scheduler = TokioScheduler::current();
//!     let ran = Arc::new(Mutex::new(false));
//!     let ran_in_task = Arc::clone(&ran);
//!
//!     // Runs the closure after 5 ms; dropping the returned disposal before that would cancel it.
//!     let _disposal = scheduler.schedule(
//!         move || *ran_in_task.lock().unwrap() = true,
//!         Some(Duration::from_millis(5)),
//!     );
//!     tokio::time::sleep(Duration::from_millis(20)).await;
//!     assert!(*ran.lock().unwrap());
//! }
//! ```
// The link to `ThreadPoolScheduler` above resolves only with `futures-scheduler`; docs.rs and CI
// build the docs with every feature, and other builds leave it as text instead of failing.
#![cfg_attr(
    not(feature = "futures-scheduler"),
    allow(rustdoc::broken_intra_doc_links)
)]

pub mod runtime;
mod task;
pub mod virtual_time;

#[cfg(feature = "futures")]
pub use task::StreamThenContext;
pub use task::{
    FutureThenContext, OnceContext, PeriodicContext, RecursiveContext, Stepper, Task, TaskHandler,
    TaskState, drive, yield_now,
};

use crate::{disposable::Disposable, observable::Subscription, thread_mode::ThreadMode};
#[cfg(feature = "futures")]
use futures::Stream;
use std::{
    future::Future,
    time::{Duration, Instant},
};

/// The part of a scheduler that does not depend on the task: the thread mode its tasks run in,
/// and the disposal that cancels one.
///
/// It is split from [`Scheduler`] because neither may depend on the task's type: a scheduler has
/// one kind of handle whatever it runs, so an operator's disposal can name `S::Disposal` without
/// naming the task, which names the operator's observer.
pub trait SchedulerTypes {
    /// The thread mode of the tasks: [`Shared`](crate::thread_mode::Shared) for a scheduler that
    /// can run them on another thread than the one that scheduled them,
    /// [`Local`](crate::thread_mode::Local) for one that runs them on the scheduling thread.
    type Mode: ThreadMode;

    /// The disposal [`Scheduler::run_task`] returns: disposing it cancels the task.
    ///
    /// A disposed task should be dropped promptly. The task of a time-based operator or of
    /// `observe_on` holds the operator's context, and a subscription disposed after its source let
    /// go of its observer releases that observer only when the task is dropped (see
    /// `docs/decisions/0004-a-source-that-drops-its-observer.md`). A task parked on its own sleep
    /// must therefore be woken to stop: the built-in schedulers wake it, so a disposed task and
    /// the observer it holds are dropped promptly, not when the timer fires.
    ///
    /// The disposal is a handle to the task, never its owner: the task's state must be dropped once
    /// the task finishes or is cancelled, whether or not its disposal is still alive. An operator
    /// keeps the disposal in the context the task holds, so a disposal that kept the task alive
    /// would keep that context, and the downstream observer, alive with it until the subscription
    /// stops. The disposals of the built-in schedulers are handles to their runtime's task.
    type Disposal: Disposable;

    /// The current time on the scheduler's clock: the system's for the built-in schedulers.
    ///
    /// Every operator that measures time reads it here, at subscription or on an event, and a
    /// task gets it as the `now` of each step, so that a scheduler with a clock of its own — a
    /// virtual one in tests — drives the deadlines and timestamps of the operators it is given.
    /// A [`TaskState::SleepUntil`] is an instant on this clock.
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Runs a [`Task`], after an optional delay, until it reports [`TaskState::Finished`].
///
/// The trait is generic over the task's states `TC` and `P` so that each implementation states
/// what it requires of them: `Send + 'static` for a multi-threaded scheduler, `'static` for a
/// single-threaded one. A trait method could not tighten its bounds per implementation.
pub trait Scheduler<TC, P = ()>: SchedulerTypes + Clone {
    /// Spawns `task`, to start after `delay`. Dropping the returned subscription cancels it.
    ///
    /// A `delay` too long for an [`Instant`] to represent never ends: the task is never stepped,
    /// and is kept until it is disposed. [`drive`] does that for the schedulers built on it.
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal>;
}

/// The closure-taking conveniences of every scheduler, built on [`Scheduler::run_task`]. A
/// multi-threaded scheduler refuses a closure that is not `Send`.
pub trait SchedulerExt: SchedulerTypes + Clone {
    /// Runs `task` once, after `delay`.
    fn schedule<F>(&self, task: F, delay: Option<Duration>) -> Subscription<Self::Disposal>
    where
        F: FnOnce(),
        Self: Scheduler<OnceContext<F>>,
    {
        self.run_task(Task::once(task, |task| task()), delay)
    }

    /// Runs `task(count, now)` until it returns [`TaskState::Finished`], as [`Task::recursive`]
    /// does. `now` is the time of the step on the scheduler's clock, the one a
    /// [`TaskState::SleepUntil`] is measured on.
    fn schedule_recursively<F>(
        &self,
        task: F,
        delay: Option<Duration>,
    ) -> Subscription<Self::Disposal>
    where
        F: FnMut(usize, Instant) -> TaskState,
        Self: Scheduler<RecursiveContext<F>>,
    {
        self.run_task(
            Task::recursive(task, |task, count, now| task(count, now)),
            delay,
        )
    }

    /// Runs `task(count)` every `period`, from now plus `delay`, until it returns `false`. Runs
    /// that fall behind are caught up back to back, never skipped.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    fn schedule_periodically<F>(
        &self,
        task: F,
        period: Duration,
        delay: Option<Duration>,
    ) -> Subscription<Self::Disposal>
    where
        F: FnMut(usize) -> bool,
        Self: Scheduler<PeriodicContext<F>>,
    {
        // An anchor out of range is a delay that never ends: the first run never comes, so it
        // needs no anchor.
        let anchor = self.now().checked_add(delay.unwrap_or_default());
        self.run_task(
            Task::periodic(task, |task, count| task(count), period, anchor),
            delay,
        )
    }

    /// Drives `future` to completion.
    fn spawn_future<FU>(&self, future: FU) -> Subscription<Self::Disposal>
    where
        FU: Future<Output = ()>,
        Self: Scheduler<FutureThenContext<(), ()>, FU>,
    {
        self.run_task(Task::from_future(future), None)
    }

    /// Drives `stream` to its end: `callback(Some(item))` for each element, then `callback(None)`.
    ///
    /// `callback` returning `false`, or a disposal, stops the stream right there, without the
    /// final `None`. The task yields after each element, even when the stream is always ready.
    #[cfg(feature = "futures")]
    fn schedule_stream<SM, F>(&self, stream: SM, callback: F) -> Subscription<Self::Disposal>
    where
        SM: Stream,
        F: FnMut(Option<SM::Item>) -> bool,
        Self: Scheduler<StreamThenContext<F, SM::Item>, SM>,
    {
        self.run_task(Task::from_stream(stream, callback), None)
    }
}

impl<S: SchedulerTypes + Clone> SchedulerExt for S {}
