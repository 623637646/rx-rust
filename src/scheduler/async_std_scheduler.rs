use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::NecessarySend,
};
use educe::Educe;
use std::time::Duration;

/// Schedules tasks using the async-std runtime utilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncStdScheduler;

/// Provides the async-std-backed `Scheduler` implementation.
impl Scheduler for AsyncStdScheduler {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<F>>
    where
        F: Future<Output = ()> + NecessarySend + 'static,
    {
        let handle = async_std::task::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static + use<> {
        async_std::task::sleep(duration)
    }
}

impl Disposable for async_std::task::JoinHandle<()> {
    fn dispose(self) {
        async_std::task::spawn(self.cancel());
    }
}
