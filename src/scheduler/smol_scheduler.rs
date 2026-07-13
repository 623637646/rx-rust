use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use educe::Educe;
use std::time::Duration;

/// Schedules tasks using the smol runtime utilities.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SmolScheduler;

/// Provides the smol-backed `Scheduler` implementation.
impl Scheduler for SmolScheduler {
    type D = smol::Task<()>;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = smol::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        smol::Timer::after(duration)
    }
}

impl Disposable for smol::Task<()> {
    fn dispose(self) {
        // Dropping a `smol::Task` cancels it.
    }
}
