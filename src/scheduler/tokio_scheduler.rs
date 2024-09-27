use super::Scheduler;
use std::time::Duration;

pub struct TokioScheduler;

impl<T> Scheduler<T, Box<dyn FnOnce()>> for TokioScheduler
where
    T: FnOnce() + Send + 'static,
{
    fn schedule(&self, task: T, delay: Option<Duration>) -> Box<dyn FnOnce()> {
        let handle = tokio::spawn(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            task();
        });
        Box::new(move || handle.abort())
    }
}
