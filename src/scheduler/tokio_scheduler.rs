use super::Scheduler;
use crate::utils::disposal::Disposal;
use std::time::Duration;

pub struct TokioScheduler;

impl Scheduler for TokioScheduler {
    fn schedule(
        &self,
        task: impl FnOnce() + Send + 'static,
        delay: Option<Duration>,
    ) -> Disposal<impl FnOnce() + Send + 'static> {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            task();
        });
        Disposal::new(move || handle.abort())
    }
}
