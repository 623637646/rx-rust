use crate::tests_utils::test_runtime::spawn;
use educe::Educe;
use rx_rust::{
    scheduler::Scheduler,
    subscription::disposable::{AutoDisposal, CallbackDisposal},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct TestScheduler;

impl Scheduler for TestScheduler {
    fn schedule_future<FU>(
        &self,
        future: FU,
        result_callback: impl FnOnce(FU::Output) + Send + 'static,
    ) -> AutoDisposal<'static>
    where
        FU: Future + Send + 'static,
    {
        let entry = EntryExitChecker::enter();

        let entry_cloned = entry.clone();
        let handle = spawn(async move {
            result_callback(future.await);
            entry_cloned.exit();
        });

        let entry_cloned = entry.clone();
        AutoDisposal::new(CallbackDisposal::new(move || {
            handle.abort();
            entry_cloned.exit();
        }))
    }

    fn sleep(duration: Duration) -> impl Future + Send {
        crate::tests_utils::test_runtime::sleep(duration)
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
struct EntryExitChecker(Arc<Mutex<Boom>>);

impl EntryExitChecker {
    fn enter() -> Self {
        Self(Arc::new(Mutex::new(Boom(false))))
    }

    fn exit(&self) {
        self.0.lock().unwrap().0 = true;
    }
}

#[derive(Educe)]
#[educe(Debug)]
struct Boom(bool);

impl Drop for Boom {
    fn drop(&mut self) {
        // We use abort() instead of assert! or panic! here,
        // because tokio will catch the panic in this case.
        // Then the tests will be passed. Only show some logs in terminal.

        // assert!(self.0); // Not working

        // Only when self.0 is false and not panicking, abort.
        if !self.0 && !std::thread::panicking() {
            std::process::abort();
        }
    }
}
