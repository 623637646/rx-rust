use crate::tests_utils::join_handle::JoinHandle;
use educe::Educe;
use futures::executor::block_on;
use rx_rust::{disposable::Disposable, scheduler::Scheduler, utils::types::NecessarySend};
use std::{cell::Cell, time::Duration};

thread_local! {
    static THREAD_NAME: Cell<&'static str> = const { Cell::new("") };
}

pub fn get_thread_name() -> &'static str {
    THREAD_NAME.with(|name| name.get())
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct TestThreadScheduler {
    name: &'static str,
}

impl TestThreadScheduler {
    pub(crate) fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Scheduler for TestThreadScheduler {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let (join_handle, future) = JoinHandle::wrape(future);
        let thread_name = self.name;
        std::thread::spawn(|| {
            THREAD_NAME.with(|name| name.set(thread_name));
            block_on(future);
        });
        join_handle
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static {
        async_io::Timer::after(duration)
    }
}
