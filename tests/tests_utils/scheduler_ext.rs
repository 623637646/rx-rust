//! Closure-taking conveniences over [`Scheduler::run_task`], for the tests only: the library's
//! callers pass a [`Task`], whose state carries what a closure would capture.

use futures::Stream;
use rx_rust::{
    disposable::dispose_on_drop::DisposeOnDrop,
    scheduler::{
        Clock, FutureThenContext, Next, OnceContext, PeriodicContext, Scheduler, SchedulerTypes,
        StreamThenContext, Task,
    },
};
use std::{
    future::Future,
    time::{Duration, Instant},
};

/// The state of [`SchedulerExt::schedule_recursively`]: the closure, and how many times it ran.
pub(crate) type RecursiveState<F> = (F, usize);

/// Runs closures and futures on any scheduler. A multi-threaded scheduler refuses a closure that is
/// not `Send`.
pub(crate) trait SchedulerExt: SchedulerTypes + Clone {
    /// Runs `task` once, after `delay`.
    fn schedule<F>(&self, task: F, delay: Option<Duration>) -> DisposeOnDrop<Self::Disposal>
    where
        F: FnOnce(),
        Self: Scheduler<OnceContext<F>>,
    {
        self.run_task(Task::once(task, |task| task()), delay)
    }

    /// Runs `task(count, now)` — `count` starting at 0 — until it returns [`Next::Finished`].
    /// `now` is the time of the step on the scheduler's clock, the one a [`Next::SleepUntil`] is
    /// measured on.
    fn schedule_recursively<F>(
        &self,
        task: F,
        delay: Option<Duration>,
    ) -> DisposeOnDrop<Self::Disposal>
    where
        F: FnMut(usize, Instant) -> Next,
        Self: Scheduler<RecursiveState<F>>,
    {
        self.run_task(
            Task::new((task, 0), |(task, count), _, _, now| {
                let next = task(*count, now);
                *count += 1;
                next
            }),
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
    ) -> DisposeOnDrop<Self::Disposal>
    where
        F: FnMut(usize) -> bool,
        Self: Scheduler<PeriodicContext<F>>,
    {
        // An anchor out of range is a delay that never ends: the first run never comes, so it
        // needs no anchor.
        let anchor = self.clock().now().checked_add(delay.unwrap_or_default());
        self.run_task(
            Task::periodic(task, |task, count| task(count), period, anchor),
            delay,
        )
    }

    /// Drives `future` to completion.
    fn spawn_future<FU>(&self, future: FU) -> DisposeOnDrop<Self::Disposal>
    where
        FU: Future<Output = ()>,
        Self: Scheduler<FutureThenContext<(), ()>, FU>,
    {
        self.run_task(Task::from_future_then((), future, |(), ()| {}), None)
    }

    /// Drives `stream` to its end: `callback(Some(item))` for each element, then `callback(None)`.
    ///
    /// `callback` returning `false`, or a disposal, stops the stream right there, without the
    /// final `None`. The task yields after each element, even when the stream is always ready.
    fn schedule_stream<SM, F>(&self, stream: SM, callback: F) -> DisposeOnDrop<Self::Disposal>
    where
        SM: Stream,
        F: FnMut(Option<SM::Item>) -> bool,
        Self: Scheduler<StreamThenContext<F, SM::Item>, SM>,
    {
        self.run_task(
            Task::from_stream_then(
                callback,
                stream,
                |callback, item| callback(Some(item)),
                |mut callback| {
                    callback(None);
                },
            ),
            None,
        )
    }
}

impl<S: SchedulerTypes + Clone> SchedulerExt for S {}
