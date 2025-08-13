use crate::tests_utils::test_runtime::TestRuntime;
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal},
    scheduler::Scheduler,
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

impl Scheduler for TestRuntime {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        let is_finished = Shared::new(AtomicBool::new(false));
        let is_finished_cloned = is_finished.clone();
        let alive_tasks_count = self.alive_tasks_count.clone();
        let alive_tasks_count_cloned = alive_tasks_count.clone();
        let entry = Shared::new(Mutable::new(EntryExitChecker::enter()));
        let entry_cloned = entry.clone();
        alive_tasks_count.fetch_add(1, Ordering::SeqCst);
        let future = async move {
            future.await;
            let _ = is_finished.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |is_finished| {
                if !is_finished {
                    entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
                    alive_tasks_count.fetch_sub(1, Ordering::SeqCst);
                    Some(true)
                } else {
                    None
                }
            });
        };
        let handle = self.spawn(future);
        CallbackDisposal::new(move || {
            let _ = is_finished_cloned.fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |is_finished| {
                    if !is_finished {
                        entry_cloned.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
                        alive_tasks_count_cloned.fetch_sub(1, Ordering::SeqCst);
                        Some(true)
                    } else {
                        None
                    }
                },
            );
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
