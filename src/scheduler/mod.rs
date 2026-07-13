#[cfg(all(feature = "async-std-scheduler", not(feature = "single-threaded")))]
pub mod async_std_scheduler;
#[cfg(feature = "local-pool-scheduler")]
pub mod local_pool_scheduler;
#[cfg(all(feature = "smol-scheduler", not(feature = "single-threaded")))]
pub mod smol_scheduler;
#[cfg(all(feature = "thread-pool-scheduler", not(feature = "single-threaded")))]
pub mod thread_pool_scheduler;
#[cfg(all(feature = "tokio-scheduler", not(feature = "single-threaded")))]
pub mod tokio_scheduler;

use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use educe::Educe;
#[cfg(feature = "futures")]
use futures::{Stream, stream::StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Indicates how a recursive scheduling step should continue.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum RecursionAction {
    ContinueAt(Instant),
    ContinueImmediately,
    Stop,
}

/// A future that yields to the executor exactly once before completing.
///
/// Runtime-agnostic replacement for `yield_now`: it guarantees an await point
/// so other tasks can make progress and disposal/abort can take effect.
/// Note that `sleep(Duration::ZERO)` is NOT a substitute — e.g. tokio's
/// `sleep` with an already-elapsed deadline completes on the first poll
/// without ever yielding.
struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// Core abstraction for driving asynchronous work across runtimes.
/// See <https://reactivex.io/documentation/scheduler.html>
/// This is why the task must be 'static: <https://stackoverflow.com/a/65287449/9315497>
pub trait Scheduler {
    type D: Disposable + MaybeSend + 'static;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D>;

    /// Returns a future that completes `duration` after this call.
    ///
    /// Contract for implementors: the deadline is captured when `sleep` is
    /// *called*, not when the returned future is first polled.
    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<Self>;

    fn schedule(
        &self,
        task: impl FnOnce() + MaybeSend + 'static,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<Self::D> {
        let delay = delay.map(|duration| self.sleep(duration));
        self.spawn_future(async move {
            if let Some(delay) = delay {
                delay.await;
            }
            task()
        })
    }

    /// Repeatedly runs `task` until it returns [`RecursionAction::Stop`].
    ///
    /// The loop yields to the executor between iterations (even for
    /// `ContinueImmediately` and already-elapsed `ContinueAt` instants), so
    /// other tasks can make progress and disposal can take effect.
    fn schedule_recursively(
        &self,
        mut task: impl FnMut(usize) -> RecursionAction + MaybeSend + 'static,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<Self::D>
    where
        Self: Clone + MaybeSend + 'static,
    {
        let delay = delay.map(|duration| self.sleep(duration));
        let self_cloned = self.clone();
        self.spawn_future(async move {
            if let Some(delay) = delay {
                delay.await;
            }
            let mut count = 0;
            loop {
                match task(count) {
                    RecursionAction::ContinueAt(at) => {
                        if let Some(delay) = at.checked_duration_since(Instant::now()) {
                            self_cloned.sleep(delay).await;
                        } else {
                            // The requested instant has already passed;
                            // still yield so the loop stays cancellable.
                            YieldNow(false).await;
                        }
                    }
                    RecursionAction::ContinueImmediately => {
                        // Yield so other tasks can run and disposal can take effect.
                        YieldNow(false).await;
                    }
                    RecursionAction::Stop => break,
                }
                count += 1;
            }
        })
    }

    /// Runs `task` at a fixed rate anchored to the time of this call
    /// (plus `delay`), until `task` returns `false`.
    ///
    /// Fixed-rate semantics: if an execution overruns `period`, missed runs
    /// are executed back-to-back to catch up — they are never skipped.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    fn schedule_periodically(
        &self,
        mut task: impl FnMut(usize) -> bool + MaybeSend + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<Self::D>
    where
        Self: Clone + MaybeSend + 'static,
    {
        assert!(!period.is_zero(), "period must be non-zero");
        let mut next_time = Instant::now() + delay.unwrap_or_default();
        self.schedule_recursively(
            move |count| {
                let r#continue = task(count);
                if r#continue {
                    next_time += period;
                    RecursionAction::ContinueAt(next_time)
                } else {
                    RecursionAction::Stop
                }
            },
            delay,
        )
    }

    #[cfg(feature = "futures")]
    fn schedule_stream<SM>(
        &self,
        stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D>
    where
        SM: Stream + MaybeSend + 'static,
    {
        self.spawn_future(async move {
            let mut stream = std::pin::pin!(stream);
            while let Some(item) = stream.next().await {
                result_callback(Some(item));
            }
            result_callback(None);
        })
    }
}
