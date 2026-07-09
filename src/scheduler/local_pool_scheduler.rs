use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::{executor::LocalSpawner, task::LocalSpawnExt};
use std::time::Duration;

/// Adapts `LocalSpawner` to the `Scheduler` trait for single-threaded pools.
impl Scheduler for LocalSpawner {
    type DisposableType = LocalSpawnerDisposal;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::DisposableType> {
        let handle = self
            .spawn_local_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(LocalSpawnerDisposal(handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

pub struct LocalSpawnerDisposal(futures::future::RemoteHandle<()>);

impl Disposable for LocalSpawnerDisposal {
    fn dispose(self) {
        // Drop to call the dispose
        drop(self.0);
    }
}
