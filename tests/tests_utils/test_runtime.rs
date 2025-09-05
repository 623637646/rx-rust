use crate::tests_utils::DURATION_POST_CREATER;
use crate::tests_utils::join_handle::JoinHandle;
use educe::Educe;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::NecessarySend;
use rx_rust::utils::types::Shared;
use std::cell::Cell;
use std::sync::atomic::AtomicUsize;

thread_local! {
    static THREAD_NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub(crate) fn get_thread_name() -> Option<&'static str> {
    THREAD_NAME.with(|name| name.get())
}

cfg_if::cfg_if! {
    if #[cfg(feature = "local-pool-scheduler")] {
        use rx_rust::utils::types::Mutable;
        use futures::executor::{LocalPool, LocalSpawner};
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pool: Shared<Mutable<LocalPool>>,
            pub(crate) spawner: LocalSpawner,
            pub(crate) alive_tasks_count: Shared<AtomicUsize>,
            pub(crate) mock_delay: bool,
            #[cfg(not(feature = "single-threaded"))]
            thread_name: Option<&'static str>,
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                let pool = LocalPool::new();
                let spawner = pool.spawner();
                Self {
                    pool: Shared::new(Mutable::new(pool)),
                    spawner,
                    alive_tasks_count: Shared::new(AtomicUsize::new(0)),
                    mock_delay: false,
                    #[cfg(not(feature = "single-threaded"))]
                    thread_name: None
                }
            }
        }
    } else if #[cfg(feature = "thread-pool-scheduler")] {
        use futures::executor::ThreadPool;
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pub(crate) pool: ThreadPool,
            pub(crate) alive_tasks_count: Shared<AtomicUsize>,
            pub(crate) mock_delay: bool,
            #[cfg(not(feature = "single-threaded"))]
            thread_name: Option<&'static str>,
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                Self {
                    pool: ThreadPool::new().unwrap(),
                    alive_tasks_count: Shared::new(AtomicUsize::new(0)),
                    mock_delay: false,
                    #[cfg(not(feature = "single-threaded"))]
                    thread_name: None
                }
            }
        }
    } else {
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pub(crate) alive_tasks_count: Shared<AtomicUsize>,
            pub(crate) mock_delay: bool,
            #[cfg(not(feature = "single-threaded"))]
            thread_name: Option<&'static str>,
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                Self {
                    alive_tasks_count: Shared::new(AtomicUsize::new(0)),
                    mock_delay: false,
                    #[cfg(not(feature = "single-threaded"))]
                    thread_name: None
                }
            }
        }
    }

}

impl TestRuntime {
    pub(crate) fn spawn<FU>(&self, future: FU) -> JoinHandle<FU>
    where
        FU: Future + NecessarySend + 'static,
        FU::Output: NecessarySend + 'static,
    {
        let (join_handle, future) = JoinHandle::wrap(future);
        let self_cloned = self.clone();
        let future = async move {
            if self_cloned.mock_delay {
                self_cloned.sleep(DURATION_POST_CREATER).await;
            }
            future.await
        };

        #[cfg(not(feature = "single-threaded"))]
        if let Some(thread_name) = self.thread_name {
            std::thread::spawn(move || {
                THREAD_NAME.with(|name| name.set(Some(thread_name)));
                block_on(|_| future);
            });
            return join_handle;
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                use futures::task::LocalSpawnExt;
                self.spawner.spawn_local(future).unwrap();
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                use futures::task::SpawnExt;
                self.pool.spawn(future).unwrap();
            } else if #[cfg(feature = "tokio-scheduler")] {
                tokio::runtime::Handle::current().spawn(future);
            } else if #[cfg(feature = "async-std-scheduler")] {
                async_std::task::spawn(future);
            } else {
                _ = future;
            }
        }
        join_handle
    }

    #[cfg(not(feature = "single-threaded"))]
    pub(crate) fn clone_with_thread_name(&self, thread_name: &'static str) -> Self {
        let mut cloned = self.clone();
        cloned.thread_name = Some(thread_name);
        cloned
    }
}

pub(crate) fn block_on<FU>(body: impl FnOnce(TestRuntime) -> FU)
where
    FU: Future<Output = ()> + 'static,
{
    let runtime: TestRuntime = Default::default();

    cfg_if::cfg_if! {
        if #[cfg(feature = "local-pool-scheduler")] {
            use futures::task::LocalSpawnExt;
            use rx_rust::utils::types::MutableHelper;
            runtime.spawner.spawn_local(body(runtime.clone())).unwrap();
            runtime.pool.lock_mut(|mut lock| lock.run());
        } else if #[cfg(feature = "thread-pool-scheduler")] {
            futures::executor::block_on(body(runtime));
        } else if #[cfg(feature = "tokio-scheduler")] {
            tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body(runtime));
        } else if #[cfg(feature = "async-std-scheduler")] {
            async_std::task::block_on(body(runtime));
        } else {
            _ = body(runtime);
            panic!("You need to specify a feature to run tests.");
        }
    }
}
