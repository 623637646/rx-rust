use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use educe::Educe;
use futures::future::abortable;
use std::time::Duration;

/// Schedules tasks using the async-std runtime utilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncStdScheduler;

/// Provides the async-std-backed `Scheduler` implementation.
impl Scheduler for AsyncStdScheduler {
    type D = AsyncStdDisposal;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let (future, abort_handle) = abortable(future);
        async_std::task::spawn(future);
        BoundDropDisposal::new(AsyncStdDisposal(abort_handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_std::task::sleep(duration)
    }
}

pub struct AsyncStdDisposal(futures::future::AbortHandle);

impl Disposable for AsyncStdDisposal {
    fn dispose(self) {
        self.0.abort();
    }
}
