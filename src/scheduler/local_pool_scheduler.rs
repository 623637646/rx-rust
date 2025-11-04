use super::Scheduler;
use crate::{disposable::Disposable, utils::types::NecessarySendSync};
use futures::{
    executor::LocalSpawner,
    stream::{AbortHandle, Abortable},
    task::LocalSpawnExt,
};
use std::time::Duration;

/// Adapts `LocalSpawner` to the `Scheduler` trait for single-threaded pools.
impl Scheduler for LocalSpawner {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySendSync + 'static,
    ) -> impl Disposable + NecessarySendSync + 'static {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = Abortable::new(future, abort_registration);
        self.spawn_local(async {
            _ = future.await;
        })
        .expect("failed to spawn future");
        abort_handle
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySendSync + 'static {
        async_io::Timer::after(duration)
    }
}
