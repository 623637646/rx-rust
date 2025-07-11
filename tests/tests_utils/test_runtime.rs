use educe::Educe;
use futures::{
    channel::oneshot::{Canceled, Receiver},
    future::abortable,
    stream::AbortHandle,
};
use pin_project::pin_project;
use rand::{
    Rng,
    distr::{Distribution, StandardUniform},
    random,
};
use rx_rust::utils::types::NecessarySend;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use futures::executor::{LocalPool, LocalSpawner};
        use rx_rust::utils::types::{Mutable, Shared, MutableHelper};
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) enum TestRuntime {
            FuturesLocalPool(Shared<Mutable<LocalPool>>, LocalSpawner),
        }
    } else {
        use futures::executor::ThreadPool;
        #[derive(Educe)]
        #[educe(Debug, Clone)]
        pub(crate) enum TestRuntime {
            FuturesThreadPool(ThreadPool),
            Tokio,
            AsyncStd,
        }
    }
}

impl Distribution<TestRuntime> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestRuntime {
        cfg_if::cfg_if! {
            if #[cfg(feature = "single-threaded")] {
                match rng.random_range(0..=0) {
                    0 => {
                        let pool = LocalPool::new();
                        let spawner = pool.spawner();
                        TestRuntime::FuturesLocalPool(Shared::new(Mutable::new(pool)), spawner)
                    }
                    _ => unreachable!(),
                }
            } else {
                match rng.random_range(0..=2) {
                    0 => TestRuntime::FuturesThreadPool(ThreadPool::new().unwrap()),
                    1 => TestRuntime::Tokio,
                    2 => TestRuntime::AsyncStd,
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl TestRuntime {
    pub(crate) fn spawn<FU>(&self, future: FU) -> JoinHandle<FU::Output>
    where
        FU: Future + NecessarySend + 'static,
        FU::Output: NecessarySend + 'static,
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        let (future, abort_handle) = abortable(future);

        let future = async {
            let result = future.await;
            if let Ok(value) = result {
                _ = tx.send(value);
            }
        };

        cfg_if::cfg_if! {
            if #[cfg(feature = "single-threaded")] {
                match &self {
                    TestRuntime::FuturesLocalPool(_, spawner) => {
                        use futures::task::LocalSpawnExt;
                        spawner.spawn_local(future).unwrap()
                    }
                }
            } else {
                use futures::task::SpawnExt;
                match &self {
                    TestRuntime::FuturesThreadPool(pool) => {
                        pool.spawn(future).unwrap();
                    }
                    TestRuntime::Tokio => {
                        tokio::runtime::Handle::current().spawn(future);
                    }
                    TestRuntime::AsyncStd => {
                        async_std::task::spawn(future);
                    }
                };
            }
        }

        JoinHandle { rx, abort_handle }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        pub(crate) fn block_on<FU>(body: impl FnOnce(TestRuntime) -> FU)
        where
            FU: Future<Output = ()> + 'static,
        {
            let runtime = random::<TestRuntime>();

            match &runtime {
                TestRuntime::FuturesLocalPool(local_pool, spawner) => {
                    use futures::task::LocalSpawnExt;
                    spawner.spawn_local(body(runtime.clone())).unwrap();
                    local_pool.lock_mut().run();
                }
            }
        }
    } else {
        pub(crate) fn block_on<FU>(body: impl FnOnce(TestRuntime) -> FU)
        where
            FU: Future<Output = ()>,
        {
            let runtime = random::<TestRuntime>();

            match &runtime {
                TestRuntime::FuturesThreadPool(_) => futures::executor::block_on(body(runtime)),
                TestRuntime::Tokio => if random() {
                    tokio::runtime::Builder::new_current_thread()
                } else {
                    tokio::runtime::Builder::new_multi_thread()
                }
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body(runtime)),
                TestRuntime::AsyncStd => async_std::task::block_on(body(runtime)),
            }
        }
    }
}

#[pin_project]
pub(crate) struct JoinHandle<T> {
    #[pin]
    rx: Receiver<T>,
    abort_handle: AbortHandle,
}

impl<T> JoinHandle<T> {
    pub(crate) fn abort(self) {
        self.abort_handle.abort();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, Canceled>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        Future::poll(this.rx, cx)
    }
}
