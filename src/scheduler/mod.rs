use std::time::Duration;

#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

/// A `Scheduler` is a type that can schedule tasks.
pub trait Scheduler {
    /// Schedule a task to be executed.
    /// task: The task to be executed. The task must be Send and 'static, because the task will be executed in a different thread.
    /// delay: The delay before the task is executed.
    /// Returns a handle that can be used to cancel the task.
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static,
        delay: Option<Duration>,
    ) -> impl FnOnce() + Send + 'static;
}
