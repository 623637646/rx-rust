use super::Scheduler;
use std::time::Duration;

pub struct TokioScheduler;

impl Scheduler for TokioScheduler {
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
