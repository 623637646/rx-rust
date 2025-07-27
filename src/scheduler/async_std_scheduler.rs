use super::Scheduler;
use crate::disposable::Disposable;
use crate::utils::types::NecessarySend;
use educe::Educe;
use futures::stream::{AbortHandle, Abortable};
use std::time::Duration;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncStdScheduler;

impl Scheduler for AsyncStdScheduler {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = Abortable::new(future, abort_registration);
        async_std::task::spawn(future);
        abort_handle
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static {
        async_std::task::sleep(duration)
    }
}
