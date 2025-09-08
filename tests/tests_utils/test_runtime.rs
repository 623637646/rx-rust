use crate::tests_utils::join_handle::JoinHandle;
use educe::Educe;
use rx_rust::utils::types::NecessarySend;
use rx_rust::utils::types::Shared;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Educe)]
#[educe(Debug, Clone, Default)]
struct TestRuntimeContext {
    alive_tasks_count: Shared<AtomicUsize>,
    #[cfg(not(feature = "single-threaded"))]
    is_spawned_late: bool,
    #[cfg(not(feature = "single-threaded"))]
    is_abort_late: bool,
    #[cfg(not(feature = "single-threaded"))]
    thread_name: Option<&'static str>,
}

cfg_if::cfg_if! {
    if #[cfg(feature = "local-pool-scheduler")] {
        use futures::executor::{LocalPool, LocalSpawner};
        use rx_rust::utils::types::Mutable;
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pool: Shared<Mutable<LocalPool>>,
            pub(crate) spawner: LocalSpawner,
            context: TestRuntimeContext
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                let pool = LocalPool::new();
                let spawner = pool.spawner();
                Self {
                    pool: Shared::new(Mutable::new(pool)),
                    spawner,
                    context: TestRuntimeContext::default(),
                }
            }
        }
    } else if #[cfg(feature = "thread-pool-scheduler")] {
        use futures::executor::ThreadPool;
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pub(crate) pool: ThreadPool,
            context: TestRuntimeContext
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                Self {
                    pool: ThreadPool::new().unwrap(),
                    context: TestRuntimeContext::default(),
                }
            }
        }
    } else {
        #[derive(Educe)]
        #[educe(Debug, Clone, Default)]
        pub(crate) struct TestRuntime {
            context: TestRuntimeContext
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
                panic!("You need to specify a feature to run tests.");
            }
        }
        join_handle
    }

    #[cfg(not(feature = "single-threaded"))]
    pub(crate) fn clone_with_thread_name(&self, thread_name: &'static str) -> Self {
        let mut cloned = self.clone();
        cloned.context.thread_name = Some(thread_name);
        cloned
    }

    pub(crate) fn get_alive_tasks_count(&self) -> usize {
        self.context.alive_tasks_count.load(Ordering::SeqCst)
    }

    pub(crate) fn alive_tasks_count_add_1(&self) {
        self.context
            .alive_tasks_count
            .fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn alive_tasks_count_sub_1(&self) {
        self.context
            .alive_tasks_count
            .fetch_sub(1, Ordering::SeqCst);
    }

    #[cfg(not(feature = "single-threaded"))]
    pub(crate) fn is_spawned_late(&self) -> bool {
        self.context.is_spawned_late
    }

    #[cfg(not(feature = "single-threaded"))]
    pub(crate) fn is_abort_late(&self) -> bool {
        self.context.is_abort_late
    }

    #[cfg(not(feature = "single-threaded"))]
    pub(crate) fn get_expected_thread_name(&self) -> Option<&'static str> {
        self.context.thread_name
    }

    pub(crate) fn mock_delay(&mut self) {
        #[cfg(not(feature = "single-threaded"))]
        {
            self.context.is_spawned_late = true;
            self.context.is_abort_late = false;
        }
    }
}

pub(crate) fn block_on<FU>(body: impl FnOnce(TestRuntime) -> FU)
where
    FU: Future<Output = ()> + 'static,
{
    let runtime = TestRuntime::default();
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
            panic!("You need to specify a feature to run tests.");
        }
    }
}
