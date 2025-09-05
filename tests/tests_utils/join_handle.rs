use futures::{
    channel::oneshot::{Canceled, Receiver},
    future::abortable,
    stream::AbortHandle,
};
use pin_project::pin_project;
use rx_rust::disposable::Disposable;

#[pin_project]
pub(crate) struct JoinHandle<FU>
where
    FU: Future,
{
    #[pin]
    rx: Receiver<FU::Output>,
    abort_handle: AbortHandle,
}

impl<FU> JoinHandle<FU>
where
    FU: Future,
{
    pub(crate) fn wrap(future: FU) -> (Self, impl Future<Output = ()>) {
        let (tx, rx) = futures::channel::oneshot::channel();
        let (future, abort_handle) = abortable(future);

        let future = async {
            let result = future.await;
            if let Ok(value) = result {
                _ = tx.send(value);
            }
        };

        (JoinHandle { rx, abort_handle }, future)
    }
}

impl<FU> Future for JoinHandle<FU>
where
    FU: Future,
{
    type Output = Result<FU::Output, Canceled>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        Future::poll(this.rx, cx)
    }
}

impl<FU> Disposable for JoinHandle<FU>
where
    FU: Future,
{
    fn dispose(self) {
        self.abort_handle.abort();
    }
}
