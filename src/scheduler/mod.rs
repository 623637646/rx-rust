use crate::subscription::disposable::AutoDisposal;
use futures::{Stream, stream::StreamExt};
use std::time::Duration;

#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

/// This is why the task must be 'static: https://stackoverflow.com/a/65287449/9315497
pub trait Scheduler {
    fn schedule_future<FU>(
        &self,
        future: FU,
        result_callback: impl FnOnce(FU::Output) + Send + 'static,
    ) -> AutoDisposal<'static>
    where
        FU: Future + Send + 'static;

    fn sleep(duration: Duration) -> impl Future + Send;

    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        self.schedule_future(
            async move {
                if let Some(delay) = delay {
                    Self::sleep(delay).await;
                }
                task();
            },
            |_| {},
        )
    }

    fn schedule_recursive(
        &self,
        mut task: impl FnMut(usize) -> Option<Duration> + Send + 'static,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        self.schedule_future(
            async move {
                if let Some(delay) = delay {
                    Self::sleep(delay).await;
                }
                let mut count = 0;
                while let Some(delay) = task(count) {
                    Self::sleep(delay).await;
                    count += 1;
                }
            },
            |_| {},
        )
    }

    fn schedule_period(
        &self,
        mut task: impl FnMut(usize) -> bool + Send + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        self.schedule_recursive(
            move |count| {
                task(count);
                Some(period)
            },
            delay,
        )
    }

    fn schedule_stream<SM>(
        &self,
        mut stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) + Send + 'static,
    ) -> AutoDisposal<'static>
    where
        SM: Stream + Send + Unpin + 'static,
    {
        self.schedule_future(
            async move {
                while let Some(item) = stream.next().await {
                    result_callback(Some(item));
                }
                result_callback(None);
            },
            |_| {},
        )
    }
}
