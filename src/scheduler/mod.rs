#[cfg(feature = "async-std-scheduler")]
pub mod async_std_scheduler;
#[cfg(feature = "local-pool-scheduler")]
pub mod local_pool_scheduler;
#[cfg(feature = "thread-pool-scheduler")]
pub mod thread_pool_scheduler;
#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

use crate::{disposable::auto_disposal::AutoDisposal, utils::types::NecessarySend};
#[cfg(feature = "futures")]
use futures::{Stream, stream::StreamExt};
use std::time::Duration;

/// This is why the task must be 'static: https://stackoverflow.com/a/65287449/9315497
pub trait Scheduler: Clone + NecessarySend + 'static {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> AutoDisposal<'static>;

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + NecessarySend;

    fn schedule(
        &self,
        task: impl FnOnce() + NecessarySend + 'static,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            task();
        })
    }

    fn schedule_recursive(
        &self,
        mut task: impl FnMut(usize) -> Option<Duration> + NecessarySend + 'static,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            let mut count = 0;
            while let Some(delay) = task(count) {
                this.sleep(delay).await;
                count += 1;
            }
        })
    }

    fn schedule_period(
        &self,
        mut task: impl FnMut(usize) -> bool + NecessarySend + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        self.schedule_recursive(
            move |count| {
                let stop = task(count);
                if stop { None } else { Some(period) }
            },
            delay,
        )
    }

    #[cfg(feature = "futures")]
    fn schedule_stream<SM>(
        &self,
        mut stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) + NecessarySend + 'static,
    ) -> AutoDisposal<'static>
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
