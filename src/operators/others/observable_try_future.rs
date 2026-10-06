//! The [`ObservableTryFuture`] adapter, behind
//! [`ObservableExt::into_try_future`](crate::observable::ObservableExt::into_try_future).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    thread_mode::mutable::MutableHelper,
    thread_mode::{Shared, ThreadMode},
    utils::lazy_subscription::LazySubscription,
};
use educe::Educe;
use std::task::{Poll, Waker};

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
pub struct ObservableTryFuture<OE>
where
    OE: ObservableTypes,
{
    #[educe(Debug(ignore))]
    subscription: LazySubscription<OE, Subscription<OE::Disposal>>,
    #[educe(Debug(ignore))]
    context: ContextPtr<OE>,
}

/// The state shared with the observer. It is always behind the thread-safe pointer, whatever the
/// source's mode: every synchronous source is `Local`, and many of them are `Send`, so a pointer
/// picked from the mode would make the future over them `!Send` and keep it out of a
/// multi-threaded executor. A source that really is bound to its thread keeps the future `!Send`
/// by itself.
type ContextPtr<OE> = <Shared as ThreadMode>::Ptr<
    ObservableTryFutureContext<<OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
>;

impl<OE> ObservableTryFuture<OE>
where
    OE: ObservableTypes,
{
    /// Creates an [`ObservableTryFuture`] over `source`;
    /// [`ObservableExt::into_try_future`](crate::observable::ObservableExt::into_try_future) is the
    /// fluent form.
    pub fn new(source: OE) -> Self {
        Self {
            subscription: LazySubscription::new(source),
            context: Shared::ptr(ObservableTryFutureContext {
                result: None,
                waker: None,
            }),
        }
    }
}

impl<OE> Unpin for ObservableTryFuture<OE> where OE: ObservableTypes {}

impl<T, E, OE> Future for ObservableTryFuture<OE>
where
    OE: Observable<ObservableTryFutureObserver<Shared, T, E>, Item = T, Error = E>,
{
    type Output = Result<Option<T>, E>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = &mut *self;
        this.subscription
            .subscribe_once(|| ObservableTryFutureObserver {
                context: this.context.clone(),
            });

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
                self.subscription.release();
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}

pub struct ObservableTryFutureObserver<M: ThreadMode, T, E> {
    context: M::Ptr<ObservableTryFutureContext<T, E>>,
}

impl<M: ThreadMode, T, E> ObservableTryFutureObserver<M, T, E> {
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

impl<M: ThreadMode, T, E> Observer<T, E> for ObservableTryFutureObserver<M, T, E> {
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
