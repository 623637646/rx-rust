use crate::tests_utils::test_runtime::TestRuntime;
use educe::Educe;
use rx_rust::{
    scheduler::Scheduler,
    subscription::disposable::{AutoDisposal, CallbackDisposal},
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::time::Duration;

impl Scheduler for TestRuntime {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> AutoDisposal<'static> {
        let entry = EntryExitChecker::enter();
        let entry_cloned = entry.clone();
        let future = async move {
            future.await;
            entry_cloned.exit();
        };
        let handle = self.spawn(future);
        AutoDisposal::new(CallbackDisposal::new(move || {
            handle.abort();
            entry.exit();
        }))
    }

    async fn sleep(&self, duration: Duration) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                self.spawner.sleep(duration).await;
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                self.0.sleep(duration).await;
            } else if #[cfg(feature = "tokio-scheduler")] {
                tokio::runtime::Handle::current().sleep(duration).await;
            } else if #[cfg(feature = "async-std-scheduler")] {
                use rx_rust::scheduler::async_std_scheduler::AsyncStdScheduler;
                AsyncStdScheduler.sleep(duration).await;
            } else {
                _ = duration;
            }
        }
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
struct EntryExitChecker(Shared<Mutable<Boom>>);

impl EntryExitChecker {
    fn enter() -> Self {
        Self(Shared::new(Mutable::new(Boom(false))))
    }

    fn exit(&self) {
        self.0.lock_mut().0 = true;
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
