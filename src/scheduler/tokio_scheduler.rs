use super::Scheduler;
use crate::subscription::disposable::{AutoDisposal, Disposable};
use educe::Educe;
use std::time::Duration;
use tokio::{task::JoinHandle, time::interval};

/// `TokioScheduler` is an implementation of the `Scheduler` trait using Tokio runtime.
///
/// This scheduler allows scheduling tasks to be executed immediately or after a specified delay
/// using Tokio's asynchronous runtime capabilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TokioScheduler;

impl Scheduler for TokioScheduler {
    /// Schedules a task for execution, optionally after a specified delay.
    ///
    /// # Arguments
    ///
    /// * `task` - A closure that represents the task to be executed.
    /// * `delay` - An optional `Duration` specifying the delay before task execution.
    ///
    /// # Returns
    ///
    /// Returns a `Disposable` which can abort the scheduled task if it hasn't started yet.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use rx_rust::scheduler::Scheduler;
    /// use rx_rust::scheduler::tokio_scheduler::TokioScheduler;
    /// use rx_rust::subscription::disposable::Disposable;
    /// #[tokio::main]
    /// async fn main() {
    ///     let scheduler = TokioScheduler;
    ///     let task = || println!("Task executed!");
    ///     let disposal = scheduler.schedule(task, Some(Duration::from_secs(1)));
    ///     disposal.dispose(); // To cancel the task before it executes:
    /// }
    /// ```
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            task();
        });
        AutoDisposal::new(handle)
    }

    fn schedule_period(
        &self,
        mut task: impl FnMut(usize) -> bool + Send + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            let mut ticker = interval(period);
            let mut count = 0;
            loop {
                ticker.tick().await;
                let stop = task(count);
                count += 1;
                if stop {
                    break;
                }
            }
        });
        AutoDisposal::new(handle)
    }

    fn schedule_future<FU>(
        &self,
        future: FU,
        result_callback: impl FnOnce(FU::Output) + Send + 'static,
    ) -> AutoDisposal<'static>
    where
        FU: Future + Send + 'static,
    {
        let handle = tokio::spawn(async {
            result_callback(future.await);
        });
        AutoDisposal::new(handle)
    }
}

impl<T> Disposable for JoinHandle<T> {
    fn dispose(self) {
        self.abort();
    }
}
