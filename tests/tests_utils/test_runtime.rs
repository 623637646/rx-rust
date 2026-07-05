use educe::Educe;
use futures::FutureExt;
use rx_rust::{scheduler::Scheduler, utils::types::NecessarySend};

cfg_if::cfg_if! {
    if #[cfg(feature = "local-pool-scheduler")] {
        pub(crate) type TestScheduler = futures::executor::LocalSpawner;
    } else if #[cfg(feature = "thread-pool-scheduler")] {
        pub(crate) type TestScheduler = futures::executor::ThreadPool;
    } else if #[cfg(feature = "tokio-scheduler")] {
        pub(crate) type TestScheduler = tokio::runtime::Handle;
    } else if #[cfg(feature = "async-std-scheduler")] {
        pub(crate) type TestScheduler = rx_rust::scheduler::async_std_scheduler::AsyncStdScheduler;
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct TestRuntime(TestScheduler);

impl TestRuntime {
    pub(crate) fn spawn<T>(
        &self,
        future: impl Future<Output = T> + NecessarySend + 'static,
    ) -> impl Future<Output = Option<T>>
    where
        T: NecessarySend + 'static,
    {
        cfg_if::cfg_if! {
            if #[cfg(feature = "local-pool-scheduler")] {
                use futures::task::LocalSpawnExt;
                self.0.spawn_local_with_handle(future).unwrap().map(Option::Some)
            } else if #[cfg(feature = "thread-pool-scheduler")] {
                use futures::task::SpawnExt;
                self.0.spawn_with_handle(future).unwrap().map(Option::Some)
            } else if #[cfg(feature = "tokio-scheduler")] {
                tokio::spawn(future).map(Result::ok)
            } else if #[cfg(feature = "async-std-scheduler")] {
                async_std::task::spawn(future).map(Option::Some)
            }
        }
    }
}

impl Scheduler for TestRuntime {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> rx_rust::disposable::bound_drop_disposal::BoundDropDisposal<
        impl rx_rust::disposable::Disposable + rx_rust::utils::types::NecessarySend + 'static + use<F>,
    >
    where
        F: Future<Output = ()> + rx_rust::utils::types::NecessarySend + 'static,
    {
        self.0.spawn_future(future)
    }

    fn sleep(
        &self,
        duration: std::time::Duration,
    ) -> impl Future + rx_rust::utils::types::NecessarySend + 'static + use<> {
        self.0.sleep(duration)
    }
}

pub(crate) fn block_on<FU>(body: impl FnOnce(TestRuntime) -> FU)
where
    FU: Future<Output = ()> + 'static,
{
    cfg_if::cfg_if! {
        if #[cfg(feature = "local-pool-scheduler")] {
            use futures::executor::LocalPool;
            use futures::task::LocalSpawnExt;
            let mut pool = LocalPool::new();
            let spawner = pool.spawner();
            spawner.spawn_local(body(TestRuntime(spawner.clone()))).unwrap();
            pool.run();
        } else if #[cfg(feature = "thread-pool-scheduler")] {
            use futures::executor::ThreadPool;
            futures::executor::block_on(body(TestRuntime(ThreadPool::new().unwrap())));
        } else if #[cfg(feature = "tokio-scheduler")] {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(async {
                    body(TestRuntime(tokio::runtime::Handle::current())).await
                });
        } else if #[cfg(feature = "async-std-scheduler")] {
            use rx_rust::scheduler::async_std_scheduler::AsyncStdScheduler;
            async_std::task::block_on(body(TestRuntime(AsyncStdScheduler)));
        }
    }
}
