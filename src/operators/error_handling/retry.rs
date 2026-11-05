//! The [`Retry`] operator, behind
//! [`ObservableExt::retry`](crate::observable::ObservableExt::retry).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, dispose_on_drop::DisposeOnDrop, either_disposal::EitherDisposal,
        option_disposal::OptionDisposal, replaceable_disposal::ReplaceableDisposal,
    },
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{subscribe_fn::SubscribeFn, trampoline::Trampoline},
};
use educe::Educe;

/// What the callback of [`Retry`] decides about an error.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum RetryAction<E, OE1> {
    /// Subscribe to this observable and keep going.
    Retry(OE1),
    /// Give up and terminate downstream with this error.
    Stop(E),
}

/// Re-subscribes after an error, to the observable the callback returns, until the callback
/// gives up with an error of its own.
/// See <https://reactivex.io/documentation/operators/retry.html>
///
/// Once the subscription is disposed, the callback is no longer called: an error that arrives
/// after that is dropped, as it is by [`Catch`](crate::operators::error_handling::catch::Catch).
///
/// An observable that fails while it is being subscribed is retried once its `subscribe` has
/// returned, not from inside it: however many times in a row a source fails synchronously, the
/// retries take the stack of one subscription. The subscription of the observable that failed is
/// disposed before the next one is subscribed.
///
/// Both cross threads in the `Shared` mode. An error that arrives on another thread while a
/// subscription is being made waits for it to return, and is retried by the thread that made it;
/// otherwise it is retried on the thread it arrived on. Each retry disposes the subscription before
/// it, the source's included, on the thread that runs the retry. So behind a source that requires
/// a `Send` observer, the error type and the source's disposal must be `Send`; the `Local` mode
/// does not require either.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{just::Just, throw::Throw},
///         error_handling::retry::{Retry, RetryAction},
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Retry::new(Throw::new("boom").with_item_type(), |_| {
///     RetryAction::Retry(Just::new(42).with_error_type())
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![42]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Retry<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Retry<OE, F> {
    /// Creates a [`Retry`] over `source`;
    /// [`ObservableExt::retry`](crate::observable::ObservableExt::retry) is the fluent form.
    pub fn new<T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Self { source, callback }
    }
}

/// The subscription of one attempt: of the source (`D`), of an observable the callback returned
/// (`D1`), or nothing once the callback gave up.
type AttemptDisposal<D, D1> = OptionDisposal<EitherDisposal<D, D1>>;

delegate_disposal!(
    Disposal<M, D, D1>,
    ReplaceableDisposal<M, AttemptDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1, F> ObservableTypes for Retry<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, F, OR> Observable<OR> for Retry<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            RetryObserver<JoinedModeOf<OE, OE1>, OR, F, <OE as ObservableTypes>::Disposal, OE1>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            RetryObserver<JoinedModeOf<OE, OE1>, OR, F, <OE as ObservableTypes>::Disposal, OE1>,
            Item = T,
            Error = E,
        >,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let replaceable_disposal = ReplaceableDisposal::default();
        let trampoline = Trampoline::default();
        let observer = RetryObserver {
            observer,
            callback: self.callback,
            replaceable_disposal: replaceable_disposal.clone(),
            resubscribe: SubscribeFn::new(),
            trampoline: trampoline.clone(),
        };
        // The source's subscription goes into the same slot as the retries', so that the first
        // retry disposes it before subscribing, as each retry disposes the one before.
        trampoline.run(
            || {
                replaceable_disposal.replace_with(|| {
                    self.source
                        .subscribe(observer)
                        .map_inner(|disposal| OptionDisposal::some(EitherDisposal::Left(disposal)))
                })
            },
            |(observer, error)| RetryObserver::retry_once(observer, error),
        );
        DisposeOnDrop::new(replaceable_disposal).map_inner_into()
    }
}

#[doc(hidden)]
pub struct RetryObserver<M: ThreadMode, OR, F, D: Disposable, OE1: ObservableTypes> {
    observer: OR,
    callback: F,
    /// Holds the subscription of the source, then of each observable the callback returned, or
    /// nothing once the callback gave up.
    replaceable_disposal: ReplaceableDisposal<M, AttemptDisposal<D, OE1::Disposal>>,
    /// Subscribes the observable the callback returned, with this observer.
    resubscribe: SubscribeFn<OE1, Self>,
    /// Where an error that arrives while this operator is subscribing waits for the subscription
    /// to return, so that retries follow one another instead of nesting.
    trampoline: Trampoline<M, (Self, OE1::Error)>,
}

impl<M, T, E, OR, D, OE1, F> Observer<T, E> for RetryObserver<M, OR, F, D, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => self.observer.on_termination(completion),
            Termination::Error(error) => {
                let trampoline = self.trampoline.clone();
                if let Some((observer, error)) = trampoline.park((self, error)) {
                    trampoline.run(
                        || observer.retry_once(error),
                        |(observer, error)| observer.retry_once(error),
                    );
                }
            }
        }
    }
}

impl<M, T, E, OR, D, OE1, F> RetryObserver<M, OR, F, D, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    /// Asks the callback about `error`, and subscribes to the observable it returns or terminates
    /// downstream with the error it gives up with.
    ///
    /// The callback is asked from the builder, which does not run once the subscription is
    /// disposed: an error that arrives then — from the source's own disposal, say — is dropped
    /// with the observer, without calling the callback.
    fn retry_once(mut self, error: E) {
        self.replaceable_disposal
            .clone()
            .replace_with(|| match (self.callback)(error) {
                RetryAction::Retry(observable) => {
                    let resubscribe = self.resubscribe;
                    resubscribe
                        .subscribe(observable, self)
                        .map_inner(|disposal| OptionDisposal::some(EitherDisposal::Right(disposal)))
                }
                RetryAction::Stop(error) => {
                    self.observer.on_termination(Termination::Error(error));
                    DisposeOnDrop::new(OptionDisposal::none())
                }
            })
    }
}
