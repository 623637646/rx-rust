use crate::tests_utils::test_runtime::TestRuntime;
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal},
    scheduler::Scheduler,
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::time::Duration;

impl Scheduler for TestRuntime {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let entry = Shared::new(Mutable::new(EntryExitChecker::enter()));
        let weak_entry = Shared::downgrade(&entry);
        let future = async move {
            future.await;
            if let Some(entry) = weak_entry.upgrade() {
                entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
            }
        };
        let handle = self.spawn(future);
        CallbackDisposal::new(move || {
            handle.abort();
            entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
        })
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static {
        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                self.spawner.sleep(duration)
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                self.0.sleep(duration)
            } else if #[cfg(feature = "tokio-scheduler")] {
                // TODO: Why this code doesn't work. Refer to: https://stackoverflow.com/q/79718285/9315497
                // tokio::runtime::Handle::current().sleep(duration)

                // Temeporary workaround
                tokio::time::sleep(duration)
            } else if #[cfg(feature = "async-std-scheduler")] {
                use rx_rust::scheduler::async_std_scheduler::AsyncStdScheduler;
                AsyncStdScheduler.sleep(duration)
            } else {
                _ = duration;
                async {}
            }
        }
    }
}

struct EntryExitChecker(bool);

impl EntryExitChecker {
    fn enter() -> Self {
        Self(false)
    }

    fn exit(&mut self) {
        self.0 = true;
    }
}

impl Drop for EntryExitChecker {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(self.0, "EntryExitChecker dropped without exit");
        }
    }
}
