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
