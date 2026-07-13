use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use std::time::Duration;

/// Leverages a Tokio runtime handle to drive scheduled tasks.
impl Scheduler for tokio::runtime::Handle {
    type D = tokio::task::JoinHandle<()>;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = self.spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        // Enter the runtime so the timer can be created outside a runtime context.
        let _guard = self.enter();
        tokio::time::sleep(duration)
    }
}

impl Disposable for tokio::task::JoinHandle<()> {
    fn dispose(self) {
        self.abort();
    }
}
