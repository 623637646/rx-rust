use crate::tests_utils::{DURATION_POST_CREATER, test_runtime::TestRuntime};
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal},
    safe_lock_option,
    scheduler::Scheduler,
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::{sync::atomic::Ordering, time::Duration};

impl Scheduler for TestRuntime {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let alive_tasks_count = self.alive_tasks_count.clone();
        let count_sub = Shared::new(Mutable::new(Some(move || {
            alive_tasks_count.fetch_sub(1, Ordering::SeqCst);
        })));
        let count_sub_cloned = count_sub.clone();

        let entry = Shared::new(Mutable::new(EntryExitChecker::enter()));
        let weak_entry = Shared::downgrade(&entry);

        let this = self.clone();
        let future = async move {
            if this.mock_delay {
                this.sleep(DURATION_POST_CREATER).await;
            }
            future.await;
            if let Some(entry) = weak_entry.upgrade() {
                entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
            }
            if let Some(count_sub) = safe_lock_option!(take: count_sub) {
                count_sub();
            }
        };

        self.alive_tasks_count.fetch_add(1, Ordering::SeqCst);
        let handle = self.spawn(future);
        CallbackDisposal::new(move || {
            entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
            if let Some(count_sub) = safe_lock_option!(take: count_sub_cloned) {
                count_sub();
            }
            handle.abort();
        })
    }

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static {
        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                self.spawner.sleep(duration)
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                self.pool.sleep(duration)
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
