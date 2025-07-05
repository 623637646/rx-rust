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

    fn sleep(duration: Duration) -> impl Future + Send {
        tokio::time::sleep(duration)
    }
}

impl<T> Disposable for JoinHandle<T> {
    fn dispose(self) {
        self.abort();
    }
}
