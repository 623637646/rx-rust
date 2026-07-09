use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::{executor::ThreadPool, task::SpawnExt};
use std::time::Duration;

/// Exposes `ThreadPool` as a multithreaded `Scheduler`.
impl Scheduler for ThreadPool {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + MaybeSend + 'static + use<F>>
    where
        F: Future<Output = ()> + MaybeSend + 'static,
    {
        let handel = self
            .spawn_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(ThreadPoolDisposal(handel))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

struct ThreadPoolDisposal<T>(futures::future::RemoteHandle<T>);

impl<T> Disposable for ThreadPoolDisposal<T> {
    fn dispose(self) {
        // Drop to call the dispose
    }
}
