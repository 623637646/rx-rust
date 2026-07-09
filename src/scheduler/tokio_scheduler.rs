use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use std::time::Duration;

/// Leverages a Tokio runtime handle to drive scheduled tasks.
impl Scheduler for tokio::runtime::Handle {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + MaybeSend + 'static + use<F>>
    where
        F: Future<Output = ()> + MaybeSend + 'static,
    {
        let handle = tokio::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        tokio::time::sleep(duration)
    }

    fn schedule_periodically<F>(
        &self,
        mut task: F,
        period: Duration,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<impl Disposable + MaybeSend + 'static + use<F>>
    where
        F: FnMut(usize) -> bool + MaybeSend + 'static,
    {
        let this = self.clone();
        self.spawn_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            let mut ticker = tokio::time::interval(period);
            let mut count = 0;
            loop {
                ticker.tick().await;
                let r#continue = task(count);
                count += 1;
                if !r#continue {
                    break;
                }
            }
        })
    }
}

impl Disposable for tokio::task::JoinHandle<()> {
    fn dispose(self) {
        self.abort();
    }
}
