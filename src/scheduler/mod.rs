#[cfg(feature = "async-std-scheduler")]
pub mod async_std_scheduler;
#[cfg(feature = "local-pool-scheduler")]
pub mod local_pool_scheduler;
#[cfg(feature = "thread-pool-scheduler")]
pub mod thread_pool_scheduler;
#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

use crate::{disposable::Disposable, utils::types::NecessarySend};
#[cfg(feature = "futures")]
use futures::{Stream, stream::StreamExt};
use std::time::{Duration, Instant};

pub enum RecursionAction {
    /// Smart delay with timing correction (recommended)
    ///
    /// Delays execution by accounting for timing drift from previous cycles.
    /// Calculates delay as: ideal_sleep_time + current_value - last_execution_end_time
    ///
    /// This corrects accumulated timing errors caused by imprecise system sleep.
    ContinueAfterRevisedDelay(Duration),

    /// Simple fixed delay
    ///
    /// Delays execution by exactly the specified duration from current time.
    /// No timing correction is applied.
    ContinueAfterFixedDelay(Duration),

    /// Continue immediately
    Continue,

    /// Stop execution
    Stop,
}

/// This is why the task must be 'static: https://stackoverflow.com/a/65287449/9315497
pub trait Scheduler: Clone + NecessarySend + 'static {
    fn schedule_future(
        self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static;

    fn sleep(self, duration: Duration) -> impl Future + NecessarySend;

    fn schedule(
        self,
        task: impl FnOnce() + NecessarySend + 'static,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySend + 'static {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            task();
        })
    }

    fn schedule_recursively(
        self,
        mut task: impl FnMut(usize) -> RecursionAction + NecessarySend + 'static,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySend + 'static {
        let this = self.clone();
        self.schedule_future(async move {
            let mut diff;
            if let Some(delay) = delay {
                let now = Instant::now();
                this.clone().sleep(delay).await;
                diff = now.elapsed() - delay;
            } else {
                diff = Duration::ZERO;
            }
            let mut count = 0;
            loop {
                let now = Instant::now();
                match task(count) {
                    RecursionAction::ContinueAfterRevisedDelay(delay) => {
                        // Using `let delay = delay - diff` will panic with `overflow when subtracting durations`.
                        let delay = delay.saturating_sub(diff);
                        this.clone().sleep(delay).await;
                        diff = now.elapsed() - delay;
                    }
                    RecursionAction::ContinueAfterFixedDelay(delay) => {
                        this.clone().sleep(delay).await;
                        diff = now.elapsed() - delay;
                    }
                    RecursionAction::Continue => {
                        diff = now.elapsed();
                    }
                    RecursionAction::Stop => {
                        break;
                    }
                }
                count += 1;
            }
        })
    }

    fn schedule_periodically(
        self,
        mut task: impl FnMut(usize) -> bool + NecessarySend + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySend + 'static {
        self.schedule_recursively(
            move |count| {
                let stop = task(count);
                if stop {
                    RecursionAction::Stop
                } else {
                    RecursionAction::ContinueAfterRevisedDelay(period)
                }
            },
            delay,
        )
    }

    #[cfg(feature = "futures")]
    fn schedule_stream<SM>(
        self,
        mut stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static
    where
        SM: Stream + NecessarySend + Unpin + 'static,
    {
        self.schedule_future(async move {
            while let Some(item) = stream.next().await {
                result_callback(Some(item));
            }
            result_callback(None);
        })
    }
}
