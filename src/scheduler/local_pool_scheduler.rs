use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::NecessarySend,
};
use futures::{executor::LocalSpawner, task::LocalSpawnExt};
use std::time::Duration;

/// Adapts `LocalSpawner` to the `Scheduler` trait for single-threaded pools.
impl Scheduler for LocalSpawner {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<F>>
    where
        F: Future<Output = ()> + NecessarySend + 'static,
    {
        let handel = self
            .spawn_local_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(LocalSpawnerDisposal(handel))
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

struct LocalSpawnerDisposal<T>(futures::future::RemoteHandle<T>);

impl<T> Disposable for LocalSpawnerDisposal<T> {
    fn dispose(self) {
        // Drop to call the dispose
        drop(self.0);
    }
}
