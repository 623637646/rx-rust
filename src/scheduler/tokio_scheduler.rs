use super::Scheduler;
use std::time::Duration;

/// `TokioScheduler` is an implementation of the `Scheduler` trait using Tokio runtime.
///
/// This scheduler allows scheduling tasks to be executed immediately or after a specified delay
/// using Tokio's asynchronous runtime capabilities.
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
    /// Returns a closure that, when called, aborts the scheduled task if it hasn't started yet.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use rx_rust::scheduler::Scheduler;
    /// use rx_rust::scheduler::tokio_scheduler::TokioScheduler;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let scheduler = TokioScheduler;
    ///     let task = || println!("Task executed!");
    ///     let cancel_handle = scheduler.schedule(task, Some(Duration::from_secs(1)));
    ///
    ///     // To cancel the task before it executes:
    ///     // cancel_handle();
    /// }
    /// ```
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static,
        delay: Option<Duration>,
    ) -> impl FnOnce() + Send + 'static {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            task();
        });
        move || handle.abort()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_schedule_with_no_delay() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = move || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        _ = scheduler.schedule(task, None);
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(
            elapsed_time < Duration::from_millis(10),
            "Task executed with unexpected delay"
        );
    }

    #[tokio::test]
    async fn test_schedule_with_delay() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = move || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        _ = scheduler.schedule(task, Some(Duration::from_millis(100)));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(
            elapsed_time >= Duration::from_millis(100),
            "Task executed with unexpected delay"
        );
    }

    #[tokio::test]
    async fn test_schedule_with_abort() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = move || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        let handle = scheduler.schedule(task, None);
        handle();
        assert!(rx.await.is_err());
        let elapsed_time = start_time.elapsed();
        assert!(
            elapsed_time < Duration::from_millis(10),
            "Task executed with unexpected delay"
        );
    }
}
