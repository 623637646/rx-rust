use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::NecessarySend,
};
use educe::Educe;
use std::time::Duration;

/// Schedules tasks using the smol runtime utilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SmolScheduler;

/// Provides the smol-backed `Scheduler` implementation.
impl Scheduler for SmolScheduler {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<F>>
    where
        F: Future<Output = ()> + NecessarySend + 'static,
    {
        let handle = smol::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static + use<> {
        smol::Timer::after(duration)
    }
}

impl Disposable for smol::Task<()> {
    fn dispose(self) {
        // Drop to call the dispose
    }
}
