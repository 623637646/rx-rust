use super::Scheduler;
use crate::{
    disposable::{Disposable, auto_disposal::AutoDisposal},
    utils::types::NecessarySend,
};
use std::time::Duration;

impl Scheduler for tokio::runtime::Handle {
    fn schedule_periodically(
        self,
        mut task: impl FnMut(usize) -> bool + NecessarySend + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> AutoDisposal<'static> {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            let mut ticker = tokio::time::interval(period);
            let mut count = 0;
            loop {
                ticker.tick().await;
                let stop = task(count);
                count += 1;
                if stop {
                    break;
                }
            }
        })
    }

    fn schedule_future(
        self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> AutoDisposal<'static> {
        AutoDisposal::new(self.spawn(future))
    }

    fn sleep(self, duration: Duration) -> impl Future + NecessarySend {
        tokio::time::sleep(duration)
    }
}

impl<T> Disposable for tokio::task::JoinHandle<T> {
    fn dispose(self) {
        self.abort();
    }
}
