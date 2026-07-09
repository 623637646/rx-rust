use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use educe::Educe;
use std::time::Duration;

/// Schedules tasks using the async-std runtime utilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncStdScheduler;

/// Provides the async-std-backed `Scheduler` implementation.
impl Scheduler for AsyncStdScheduler {
    type DisposableType = async_std::task::JoinHandle<()>;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::DisposableType> {
        let handle = async_std::task::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_std::task::sleep(duration)
    }
}

impl Disposable for async_std::task::JoinHandle<()> {
    fn dispose(self) {
        async_std::task::spawn(self.cancel());
    }
}
