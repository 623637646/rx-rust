use educe::Educe;
use futures::future::abortable;
use futures::stream::AbortHandle;
use rx_rust::disposable::Disposable;
use rx_rust::observable::Subscription;
use rx_rust::scheduler::{Scheduler, SchedulerTypes, Task, drive};
use rx_rust::thread_mode::Shared;
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

impl SchedulerTypes for ThreadCheckerScheduler {
    type Mode = Shared;
    type D = ThreadCheckerDisposal;
}

impl<TC, P> Scheduler<TC, P> for ThreadCheckerScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::D> {
        let thread_name = self.name;
        let future = drive(task, delay, |duration| async move {
            async_io::Timer::after(duration).await;
        });
        let (abortable, abort_handle) = abortable(future);
        std::thread::spawn(move || {
            THREAD_NAME.with(|name| name.set(Some(thread_name)));
            let _ = futures::executor::block_on(abortable);
        });
        Subscription::new(ThreadCheckerDisposal(abort_handle))
    }
}
