//! What an observable becomes as a `Future` of its first item: [`ObservableTryFuture`], behind
//! [`ObservableExt::into_try_future`](crate::observable::ObservableExt::into_try_future), and
//! [`ObservableFuture`] for a source that cannot fail, behind
//! [`ObservableExt::into_future`](crate::observable::ObservableExt::into_future).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::{Shared, ThreadMode, mutable::MutableHelper},
    utils::lazy_subscription::LazySubscription,
};
use educe::Educe;
use std::{
    convert::Infallible,
    task::{Poll, Waker},
};

struct ObservableTryFutureContext<T, E> {
    result: Option<Result<Option<T>, E>>,
    waker: Option<Waker>,
}

/// A `Future` of the first item of an observable.
///
/// It resolves with `Ok(Some(item))` on the first item, stopping the source right there, with
/// `Ok(None)` when the source completes without one, and with `Err(error)` when the source
/// fails first. That makes its output the `Maybe` of ReactiveX; an operator that always emits,
/// such as `collect` or `reduce`, in front of it gives a `Single`, and `last` picks the last item
/// instead of the first. A source that cannot fail goes through
/// [`ObservableFuture`] instead.
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
///
/// # A source that never answers
///
/// The future stays pending until the source sends an item or a termination, so a source that
/// sends neither keeps it pending forever. That is the case of a source that falls silent while it
/// holds its observer, and also of one that lets go of the observer without a termination, as
/// [`Never`](crate::operators::creating::never::Never) does at once: letting go says that nothing
/// more will come, but not whether the stream succeeded, so the future does not resolve on the
/// source's behalf (see `docs/decisions/0004-a-source-that-drops-its-observer.md`).
///
/// To bound the wait, put a [`timeout`](crate::observable::ObservableExt::timeout) in front of it:
/// it fires on such a source too, and the future then resolves with
/// `Err(timeout::Error::Timeout)`. A runtime's own timeout, such as `tokio::time::timeout`, works
/// as well.
///
/// ```rust
/// use futures::FutureExt;
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::never::Never, utility::timeout::Error},
///     scheduler::virtual_time::VirtualTime,
/// };
/// use std::time::Duration;
///
/// let time = VirtualTime::new();
/// let mut future = Never
///     .with_item_type::<i32>()
///     .timeout(Duration::from_secs(1), time.scheduler())
///     .into_try_future();
/// assert_eq!((&mut future).now_or_never(), None); // Subscribes; still pending.
///
/// time.advance_by(Duration::from_secs(1));
/// assert_eq!(future.now_or_never(), Some(Err(Error::Timeout)));
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableTryFuture<OE>
where
    OE: ObservableTypes,
{
    #[educe(Debug(ignore))]
    subscription: LazySubscription<OE, DisposeOnDrop<OE::Disposal>>,
    #[educe(Debug(ignore))]
    context: ContextPtr<OE::Item, OE::Error>,
}

/// The state the future shares with its observer. It is always behind the thread-safe pointer,
/// whatever the source's mode, so neither the future nor its observer is generic over one: every
/// synchronous source is `Local`, and many of them are `Send`, so a pointer picked from the mode
/// would make the future over them `!Send` and keep it out of a multi-threaded executor. A source
/// that really is bound to its thread keeps the future `!Send` by itself. On one thread the lock is
/// never contended, and the observer is the crate's own, so the mode has nothing else to decide.
type ContextPtr<T, E> = <Shared as ThreadMode>::Ptr<ObservableTryFutureContext<T, E>>;

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
    OE: Observable<ObservableTryFutureObserver<T, E>, Item = T, Error = E>,
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

// Not generic over a mode: the context it shares with the future is always `Shared`'s, see
// `ContextPtr`.
#[doc(hidden)]
pub struct ObservableTryFutureObserver<T, E> {
    context: ContextPtr<T, E>,
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

/// A `Future` of the first item of an observable that cannot fail.
///
/// It resolves with `Some(item)` on the first item, stopping the source right there, and with
/// `None` when the source completes without one. This is
/// [`ObservableTryFuture`] without the error it can never carry; see there for how it subscribes
/// and when it releases the source.
///
/// A source that never sends an item or a completion keeps it pending forever, `Never` among them;
/// see [`ObservableTryFuture`]. A [`timeout`](crate::observable::ObservableExt::timeout) in front
/// bounds the wait, and makes the source fallible, so the future of that one is an
/// [`into_try_future`](crate::observable::ObservableExt::into_try_future).
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
///         operators::creating::{empty::Empty, from_iter::FromIter},
///     };
///
///     let first = FromIter::new([10, 20, 30]).into_future().await;
///     assert_eq!(first, Some(10));
///
///     let none = Empty.with_item_type::<i32>().into_future().await;
///     assert_eq!(none, None);
/// }
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableFuture<OE>
where
    OE: ObservableTypes<Error = Infallible>,
{
    #[educe(Debug(ignore))]
    future: ObservableTryFuture<OE>,
}

impl<OE> ObservableFuture<OE>
where
    OE: ObservableTypes<Error = Infallible>,
{
    /// Creates an [`ObservableFuture`] over `source`;
    /// [`ObservableExt::into_future`](crate::observable::ObservableExt::into_future) is the fluent
    /// form.
    pub fn new(source: OE) -> Self {
        Self {
            future: ObservableTryFuture::new(source),
        }
    }
}

impl<OE> Unpin for ObservableFuture<OE> where OE: ObservableTypes<Error = Infallible> {}

impl<T, OE> Future for ObservableFuture<OE>
where
    OE: Observable<ObservableTryFutureObserver<T, Infallible>, Item = T, Error = Infallible>,
{
    type Output = Option<T>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        std::pin::Pin::new(&mut self.future).poll(cx).map(|result| {
            let Ok(value) = result;
            value
        })
    }
}
