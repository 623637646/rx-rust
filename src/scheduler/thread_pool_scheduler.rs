use super::Scheduler;
use crate::{disposable::Disposable, utils::types::NecessarySend};
use futures::{
    executor::ThreadPool,
    stream::{AbortHandle, Abortable},
    task::SpawnExt,
};
use std::time::Duration;

impl Scheduler for ThreadPool {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = Abortable::new(future, abort_registration);
        self.spawn(async {
            _ = future.await;
        })
        .expect("failed to spawn future");
        abort_handle
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static {
        async_io::Timer::after(duration)
    }
}
