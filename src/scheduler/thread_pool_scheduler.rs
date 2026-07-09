use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::{executor::ThreadPool, task::SpawnExt};
use std::time::Duration;

/// Exposes `ThreadPool` as a multithreaded `Scheduler`.
impl Scheduler for ThreadPool {
    type D = ThreadPoolDisposal;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = self
            .spawn_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(ThreadPoolDisposal(handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

pub struct ThreadPoolDisposal(futures::future::RemoteHandle<()>);

impl Disposable for ThreadPoolDisposal {
    fn dispose(self) {
        // Drop to call the dispose
        drop(self.0);
    }
}
