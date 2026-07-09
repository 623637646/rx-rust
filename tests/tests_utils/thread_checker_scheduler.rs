use educe::Educe;
use futures::future::abortable;
use futures::stream::AbortHandle;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::bound_drop_disposal::BoundDropDisposal;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::MaybeSend;
use std::cell::Cell;
use std::time::Duration;

thread_local! {
    static THREAD_NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub(crate) fn get_thread_name() -> Option<&'static str> {
    THREAD_NAME.with(|name| name.get())
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ThreadCheckerScheduler {
    name: &'static str,
}

impl ThreadCheckerScheduler {
    pub(crate) fn new(name: &'static str) -> Self {
        Self { name }
    }
}

pub(crate) struct ThreadCheckerDisposal(AbortHandle);

impl Disposable for ThreadCheckerDisposal {
    fn dispose(self) {
        self.0.abort();
    }
}

impl Scheduler for ThreadCheckerScheduler {
    type DisposableType = ThreadCheckerDisposal;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::DisposableType> {
        let thread_name = self.name;
        let (abortable, abort_handle) = abortable(future);
        std::thread::spawn(move || {
            THREAD_NAME.with(|name| name.set(Some(thread_name)));
            let _ = futures::executor::block_on(abortable);
        });
        BoundDropDisposal::new(ThreadCheckerDisposal(abort_handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}
