use crate::tests_utils::join_handle::JoinHandle;
use educe::Educe;
use rx_rust::utils::types::NecessarySend;

cfg_if::cfg_if! {
    if #[cfg(feature = "local-pool-scheduler")] {
        use rx_rust::utils::types::{Shared, Mutable};
        use futures::executor::{LocalPool, LocalSpawner};
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime {
            pool: Shared<Mutable<LocalPool>>,
            pub(crate) spawner: LocalSpawner,
        }
        impl Default for TestRuntime {
            fn default() -> Self {
                let pool = LocalPool::new();
                let spawner = pool.spawner();
                Self {
                    pool: Shared::new(Mutable::new(pool)),
                    spawner,
                }
            }
        }
    } else if #[cfg(feature = "thread-pool-scheduler")] {
        use futures::executor::ThreadPool;
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime(pub(crate) ThreadPool);
        impl Default for TestRuntime {
            fn default() -> Self {
                Self(ThreadPool::new().unwrap())
            }
        }
    } else {
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) struct TestRuntime;
        impl Default for TestRuntime {
            fn default() -> Self {
                Self
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
        let (join_handle, future) = JoinHandle::wrape(future);
        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                use futures::task::LocalSpawnExt;
                self.spawner.spawn_local(future).unwrap();
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                use futures::task::SpawnExt;
                self.0.spawn(future).unwrap();
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
            runtime.pool.lock_mut().run();
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
