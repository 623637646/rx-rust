use crate::subscription::disposable::AutoDisposal;
use futures::{Stream, stream::StreamExt};
use std::time::Duration;

#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

/// A `Scheduler` is a type that can schedule tasks.
pub trait Scheduler {
    /// Schedule a task to be executed.
    /// task: The task to be executed. The task must be Send and 'static, because the task will be executed in a different thread.
    /// delay: The delay before the task is executed.
    /// Returns a `Disposable` that can be used to cancel the task.
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static, // This is why the task must be 'static: https://stackoverflow.com/a/65287449/9315497
        delay: Option<Duration>,
    ) -> AutoDisposal<'static>;

    fn schedule_period(
        &self,
        task: impl FnMut(usize) -> bool + Send + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static>;

    fn schedule_future<FU>(
        &self,
        future: FU,
        result_callback: impl FnOnce(FU::Output) + Send + 'static,
    ) -> AutoDisposal<'static>
    where
        FU: Future + Send + 'static;

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
