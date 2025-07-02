use educe::Educe;
use futures::{
    channel::oneshot::{Canceled, Receiver},
    executor::ThreadPool,
    future::abortable,
    stream::AbortHandle,
    task::SpawnExt,
};
use pin_project::pin_project;
use rand::{
    Rng,
    distr::{Distribution, StandardUniform},
    random,
};
use std::sync::LazyLock;
use std::time::Duration;

static CONFIG: LazyLock<TestRuntime> = LazyLock::new(random);

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) enum TestRuntime {
    FuturesExecutor(ThreadPool),
    Tokio,
    AsyncStd,
}

impl Distribution<TestRuntime> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestRuntime {
        match rng.random_range(0..=2) {
            0 => TestRuntime::FuturesExecutor(ThreadPool::new().unwrap()),
            1 => TestRuntime::Tokio,
            2 => TestRuntime::AsyncStd,
            _ => unreachable!(),
        }
    }
}

pub(crate) fn block_on<F: Future>(body: F) -> F::Output {
    match &*CONFIG {
        TestRuntime::FuturesExecutor(_) => futures::executor::block_on(body),
        TestRuntime::Tokio => if random() {
            tokio::runtime::Builder::new_current_thread()
        } else {
            tokio::runtime::Builder::new_multi_thread()
        }
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body),
        TestRuntime::AsyncStd => async_std::task::block_on(body),
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

pub(crate) fn spawn<FU, T>(future: FU) -> JoinHandle<T>
where
    T: Send + 'static,
    FU: Future<Output = T> + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    let (future, abort_handle) = abortable(future);

    let future = async {
        let result = future.await;
        if let Ok(value) = result {
            _ = tx.send(value);
        }
    };

    match &*CONFIG {
        TestRuntime::FuturesExecutor(thread_pool) => {
            thread_pool.spawn(future).unwrap();
        }
        TestRuntime::Tokio => {
            tokio::spawn(future);
        }
        TestRuntime::AsyncStd => {
            // let handle = async_std::task::spawn(future);
            // BoxedDisposal::new(CallbackDisposal::new(move || {
            //     async_std::task::block_on(handle.cancel()); // TODO: do it like Tokio
            // }))
            todo!()
        }
    }
    JoinHandle { rx, abort_handle }
}

// TODO: 和 Scheduler里的sleep重复了。
pub(crate) async fn sleep(duration: Duration) {
    match &*CONFIG {
        TestRuntime::FuturesExecutor(_) => {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                tx.send(()).unwrap();
            });
            rx.await.unwrap();
        }
        TestRuntime::Tokio => tokio::time::sleep(duration).await,
        TestRuntime::AsyncStd => async_std::task::sleep(duration).await,
    }
}
