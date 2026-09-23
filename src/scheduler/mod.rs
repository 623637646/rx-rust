//! Running work later, or elsewhere: the [`Scheduler`] trait and its runtime adapters.
//!
//! Every time-based operator (`delay`, `debounce`, `timeout`, `interval`, …) and every operator
//! that moves work between threads (`observe_on`, `subscribe_on`) takes a value implementing
//! [`Scheduler`], so nothing is global and one program can drive different pipelines on different
//! runtimes. The adapters are selected by feature flag:
//!
//! | Feature                 | Scheduler value                          | Module                  |
//! |-------------------------|------------------------------------------|-------------------------|
//! | `tokio-scheduler`       | `tokio::runtime::Handle`                 | [`tokio_scheduler`]     |
//! | `async-std-scheduler`   | `async_std_scheduler::AsyncStdScheduler` | `async_std_scheduler`   |
//! | `smol-scheduler`        | `smol_scheduler::SmolScheduler`          | `smol_scheduler`        |
//! | `thread-pool-scheduler` | `futures::executor::ThreadPool`          | `thread_pool_scheduler` |
//! | `local-pool-scheduler`  | `futures::executor::LocalSpawner`        | `local_pool_scheduler`  |
//!
//! Each module is compiled only with its feature; these docs are built with `tokio-scheduler`.
//!
//! Implementing the trait for another runtime takes two methods: [`Scheduler::spawn_future`] and
//! [`Scheduler::sleep`]; everything else is derived from them.
//!
//! # Examples
//! ```rust
//! # #[cfg(not(feature = "tokio-scheduler"))]
//! # fn main() {}
//! # #[cfg(feature = "tokio-scheduler")]
//! #[tokio::main]
//! async fn main() {
//!     use rx_rust::scheduler::Scheduler;
//!     use std::{sync::{Arc, Mutex}, time::Duration};
//!
//!     let scheduler = tokio::runtime::Handle::current();
//!     let ran = Arc::new(Mutex::new(false));
//!     let ran_in_task = Arc::clone(&ran);
//!
//!     // Runs `task` after 5 ms; dropping the returned disposal before that would cancel it.
//!     let _disposal = scheduler.schedule(
//!         move || *ran_in_task.lock().unwrap() = true,
//!         Some(Duration::from_millis(5)),
//!     );
//!     tokio::time::sleep(Duration::from_millis(20)).await;
//!     assert!(*ran.lock().unwrap());
//! }
//! ```

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

/// What one step of [`Scheduler::schedule_recursively`] asks for next.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum RecursionAction {
    /// Run the next step at this instant, or right away if it has passed.
    ContinueAt(Instant),
    /// Run the next step as soon as the executor gets around to it.
    ContinueImmediately,
    /// This was the last step.
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

/// An executor that runs futures and tells the time.
/// See <https://reactivex.io/documentation/scheduler.html>.
///
/// Everything scheduled must be `'static`, because the runtime owns it once it is spawned
/// (<https://stackoverflow.com/a/65287449/9315497>). Only [`spawn_future`](Self::spawn_future)
/// and [`sleep`](Self::sleep) have to be implemented.
pub trait Scheduler {
    /// The handle that cancels a spawned task when disposed.
    type D: Disposable + MaybeSend + 'static;

    /// Spawns `future` on the runtime, returning a handle that cancels it when dropped.
    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D>;

    /// Returns a future that completes `duration` after this call.
    ///
    /// Contract for implementors: the deadline is captured when `sleep` is *called*, not when the
    /// returned future is first polled.
    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<Self>;

    /// Runs `task` once, after `delay` if given. Dropping the returned handle before then cancels
    /// it.
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

    /// Drives `stream` to completion, invoking `result_callback` with
    /// `Some(item)` for each element and a final `None` when the stream ends.
    ///
    /// The callback's answer is what keeps the stream running: returning
    /// `false` stops polling it right there, and the final `None` is then
    /// never delivered — the stream is dropped along with the task.
    ///
    /// Disposal aborts the task without delivering the final `None`.
    ///
    /// The loop yields to the executor after each element (even when the
    /// stream is always ready), so other tasks can make progress and
    /// disposal can take effect.
    #[cfg(feature = "futures")]
    fn schedule_stream<SM>(
        &self,
        stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) -> bool + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D>
    where
        SM: Stream + MaybeSend + 'static,
    {
        self.spawn_future(async move {
            let mut stream = std::pin::pin!(stream);
            loop {
                // A `while let` would keep the `Option<Item>` temporary alive
                // across the yield below, requiring `SM::Item: Send`.
                match stream.next().await {
                    Some(item) => {
                        if !result_callback(Some(item)) {
                            return;
                        }
                    }
                    None => break,
                }
                // Yield so other tasks can run and disposal can take effect,
                // even when the stream is always ready.
                YieldNow(false).await;
            }
            let _ = result_callback(None);
        })
    }
}
