use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    utils::mutable::{Mutable, MutableHelper},
    utils::types::{MaybeSend, Shared},
};
use educe::Educe;
use std::task::{Poll, Waker};

#[derive(Educe)]
#[educe(Debug)]
struct ObservableTryFutureContext<T, E> {
    result: Option<Result<Option<T>, E>>,
    waker: Option<Waker>,
}

/// A `Future` of the first item of an Observable.
///
/// It resolves with `Ok(Some(item))` on the first item, stopping the source right there, with
/// `Ok(None)` when the source completes without one, and with `Err(error)` when the source
/// fails first. That makes its output the `Maybe` of ReactiveX; an operator that always emits,
/// such as `collect` or `reduce`, in front of it gives a `Single`, and `last` picks the last item
/// instead of the first. A source that cannot fail goes through
/// [`ObservableFuture`](crate::operators::others::observable_future::ObservableFuture) instead.
///
/// Like any future, it does nothing until it is polled: that first poll is what subscribes to the
/// source. The subscription is dropped as soon as the future resolves, and dropping the future
/// before that disposes it.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::ObservableExt,
///         operators::creating::{from_iter::FromIter, throw::Throw},
///     };
///
///     let first = FromIter::new([10, 20, 30])
///         .with_error_type::<&str>()
///         .into_try_future()
///         .await;
///     assert_eq!(first, Ok(Some(10)));
///
///     let failed = Throw::new("boom").with_item_type::<i32>().into_try_future().await;
///     assert_eq!(failed, Err("boom"));
/// }
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableTryFuture<'or, T, E, OE>
where
    OE: Observable<'or, T, E>,
{
    source: Option<OE>,
    sub: Option<Subscription<OE::D>>,
    context: Shared<Mutable<ObservableTryFutureContext<T, E>>>,
}

impl<'or, T, E, OE> ObservableTryFuture<'or, T, E, OE>
where
    OE: Observable<'or, T, E>,
{
    pub fn new(source: OE) -> Self {
        Self {
            source: Some(source),
            sub: None,
            context: Shared::new(Mutable::new(ObservableTryFutureContext {
                result: None,
                waker: None,
            })),
        }
    }
}

impl<'or, T, E, OE> Unpin for ObservableTryFuture<'or, T, E, OE> where OE: Observable<'or, T, E> {}

impl<'or, T, E, OE> Future for ObservableTryFuture<'or, T, E, OE>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type Output = Result<Option<T>, E>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(source) = self.source.take() {
            let observer = ObservableTryFutureObserver {
                context: self.context.clone(),
            };
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        // The waker this one replaces is handed back, because dropping a `Waker` runs the
        // external code of its vtable, which must not run under the lock. Once the result is in,
        // no waker is kept: nothing is left to wake.
        let (result, previous_waker) =
            self.context
                .with_mut(|context| match context.result.take() {
                    Some(result) => (Some(result), context.waker.take()),
                    None => (None, context.waker.replace(cx.waker().clone())),
                });
        drop(previous_waker); // Drop outside the lock to avoid potential deadlock
        match result {
            Some(result) => {
                // The future is over, so the source is released now instead of whenever the
                // future itself is dropped.
                self.sub = None;
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}

struct ObservableTryFutureObserver<T, E> {
    context: Shared<Mutable<ObservableTryFutureContext<T, E>>>,
}

impl<T, E> ObservableTryFutureObserver<T, E> {
    fn resolve(&self, result: Result<Option<T>, E>) {
        // The waker is taken under the lock and woken after it is released, because waking runs
        // external code, which must not run under the lock. The result this one replaces is
        // dropped outside it too; there is none unless the source breaks its contract, since a
        // stopped or terminated observer receives nothing more.
        let (waker, replaced) = self
            .context
            .with_mut(|context| (context.waker.take(), context.result.replace(result)));
        drop(replaced);
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T, E> Observer<T, E> for ObservableTryFutureObserver<T, E> {
    fn on_next(&mut self, value: T) -> Flow {
        self.resolve(Ok(Some(value)));
        // The first item is all the future wants: the source stops here and does not terminate
        // this observer, which has already resolved the future.
        Flow::Stop
    }

    fn on_termination(self, termination: Termination<E>) {
        self.resolve(match termination {
            Termination::Completed => Ok(None),
            Termination::Error(error) => Err(error),
        });
    }
}
