use crate::subscription::disposable::Disposable;
use std::time::Duration;

#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

/// A `Scheduler` is a type that can schedule tasks.
pub trait Scheduler: Clone {
    /// Schedule a task to be executed.
    /// task: The task to be executed. The task must be Send and 'static, because the task will be executed in a different thread.
    /// delay: The delay before the task is executed.
    /// Returns a `Disposable` that can be used to cancel the task.
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static, // This is why the task must be 'static: https://stackoverflow.com/a/65287449/9315497 // TODO: use Future instead of FnOnce?
        delay: Option<Duration>,
    ) -> impl Disposable + Send + 'static;

    fn schedule_period(
        &self,
        task: impl FnMut(usize) + Send + 'static, // TODO: use Future instead of FnOnce?
        period: Duration,
        delay: Option<Duration>,
    ) -> impl Disposable + Send + 'static;
}
