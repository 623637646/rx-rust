use std::time::Duration;

#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

/// A `Scheduler` is a type that can schedule tasks.
/// Scheduler must be Send because the scheduler will be used in different threads.
/// The scheduler must be 'static because it may be stored in somewhere.
pub trait Scheduler<T, H>
where
    T: FnOnce(),
    H: FnOnce(),
{
    /// Schedule a task to be executed.
    /// task: The task to be executed. The task must be Send and 'static, because the task will be executed in a different thread.
    /// delay: The delay before the task is executed.
    /// Returns a `Disposal` that can be used to cancel the task.
    fn schedule(&self, task: T, delay: Option<Duration>) -> H;
}
