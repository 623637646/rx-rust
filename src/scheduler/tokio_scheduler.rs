use super::Scheduler;
use crate::subscription::disposable::{CallbackDisposal, Disposable};
use educe::Educe;
use std::time::Duration;
use tokio::time::interval;

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
    ) -> impl Disposable + Send + 'static {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            task();
        });
        CallbackDisposal::new(move || handle.abort())
    }

    fn schedule_period(
        &self,
        mut task: impl FnMut(usize) -> bool + Send + 'static, // TODO: use Future instead of FnOnce?
        period: Duration,
        delay: Option<Duration>,
    ) -> impl Disposable + Send + 'static {
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
        CallbackDisposal::new(move || handle.abort())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_schedule_without_delay() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        scheduler.schedule(task, None);
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_schedule_with_delay() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        scheduler.schedule(task, Some(Duration::from_millis(100)));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_schedule_with_abort() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = tokio::time::Instant::now();
        let disposal = scheduler.schedule(task, None);
        disposal.dispose();
        assert!(rx.await.is_err());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_schedule_with_late_abort() {
        let scheduler = TokioScheduler;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let disposal = scheduler.schedule(task, None);
        tokio::time::sleep(Duration::from_millis(10)).await;
        disposal.dispose();
        assert!(rx.await.is_ok());
    }

    #[tokio::test]
    async fn test_schedule_period_with_delay() {
        let scheduler = TokioScheduler;
        let counter = Arc::new(Mutex::new(Vec::new()));
        let counter_cloned = counter.clone();
        let task = move |count| {
            counter_cloned.lock().unwrap().push(count);
            false
        };
        let disposal = scheduler.schedule_period(
            task,
            Duration::from_millis(100),
            Some(Duration::from_millis(100)),
        );
        assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

        disposal.dispose();
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);
    }

    #[tokio::test]
    async fn test_schedule_period_without_delay() {
        let scheduler = TokioScheduler;
        let counter = Arc::new(Mutex::new(Vec::new()));
        let counter_cloned = counter.clone();
        let task = move |count| {
            counter_cloned.lock().unwrap().push(count);
            false
        };
        let disposal = scheduler.schedule_period(task, Duration::from_millis(100), None);
        assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

        disposal.dispose();
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);
    }

    #[tokio::test]
    async fn test_schedule_period_stop() {
        let scheduler = TokioScheduler;
        let counter = Arc::new(Mutex::new(Vec::new()));
        let counter_cloned = counter.clone();
        let task = move |count| {
            counter_cloned.lock().unwrap().push(count);
            count == 2
        };
        scheduler.schedule_period(task, Duration::from_millis(100), None);
        assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);
    }

    #[test]
    fn test_clone() {
        let scheduler = TokioScheduler;
        let _ = scheduler.clone();
    }
}
