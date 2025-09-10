use crate::tests_utils::test_runtime::TestRuntime;
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal},
    safe_lock_option,
    scheduler::Scheduler,
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::{cell::Cell, time::Duration};

thread_local! {
    static THREAD_NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub(crate) fn get_thread_name() -> Option<&'static str> {
    THREAD_NAME.with(|name| name.get())
}

impl Scheduler for TestRuntime {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySend + 'static,
    ) -> impl Disposable + NecessarySend + 'static {
        // Use this Fn() to avoid sub multiple times.
        let self_cloned = self.clone();
        let count_sub = Shared::new(Mutable::new(Some(move || {
            self_cloned.alive_tasks_count_sub_1();
        })));
        let count_sub_cloned = count_sub.clone();

        let entry = Shared::new(Mutable::new(EntryExitChecker::enter()));
        let weak_entry = Shared::downgrade(&entry);

        #[cfg(not(feature = "single-threaded"))]
        let self_cloned = self.clone();
        let future = async move {
            #[cfg(not(feature = "single-threaded"))]
            if self_cloned.is_spawned_late() {
                use crate::tests_utils::DURATION_1_MS;
                if self_cloned.get_expected_thread_name().is_none() {
                    self_cloned.sleep(DURATION_1_MS).await;
                } else {
                    std::thread::sleep(DURATION_1_MS);
                }
            }
            future.await;
            if let Some(entry) = weak_entry.upgrade() {
                entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
            }
            if let Some(count_sub) = safe_lock_option!(take: count_sub) {
                count_sub();
            }
        };

        self.alive_tasks_count_add_1();

        let handle;
        cfg_if::cfg_if! {
            if #[cfg(not(feature = "single-threaded"))] {
                if let Some(thread_name) = self.get_expected_thread_name() {
                    use crate::tests_utils::join_handle::JoinHandle;
                    let (join_handle, future) = JoinHandle::wrap(future);
                    std::thread::spawn(move || {
                        THREAD_NAME.with(|name| name.set(Some(thread_name)));
                        futures::executor::block_on(future);
                    });
                    handle = join_handle;
                } else {
                    handle = self.spawn(future);
                }
            } else {
                handle = self.spawn(future);
            }
        }

        #[cfg(not(feature = "single-threaded"))]
        let self_cloned = self.clone();
        CallbackDisposal::new(move || {
            entry.lock_mut(|mut lock| EntryExitChecker::exit(&mut lock));
            if let Some(count_sub) = safe_lock_option!(take: count_sub_cloned) {
                count_sub();
            }
            cfg_if::cfg_if! {
                if #[cfg(not(feature = "single-threaded"))] {
                    if self_cloned.is_abort_late() {
                        use crate::tests_utils::DURATION_1_MS;
                        if self_cloned.get_expected_thread_name().is_none() {
                            self_cloned.clone().spawn(async move {
                                self_cloned.sleep(DURATION_1_MS).await;
                                handle.dispose();
                            });
                        } else {
                            std::thread::spawn(move || {
                                std::thread::sleep(DURATION_1_MS);
                                handle.dispose();
                            });
                        }
                    } else {
                        handle.dispose();
                    }
                } else {
                    handle.dispose();
                }
            }
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
                panic!("You need to specify a feature to run tests.");
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
