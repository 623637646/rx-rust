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
    thread_mode::{
        ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
    utils::{on_panic::on_panic, resubscribe::Resubscribe},
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
            resubscribe: Resubscribe::new(),
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
            RetryObserver::retry_once,
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
    resubscribe: Resubscribe<OE1, Self>,
    /// Where an error that arrives while this operator is subscribing waits for the subscription
    /// to return, so that retries follow one another instead of nesting.
    trampoline: Trampoline<M, Self, OE1::Error>,
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
                if let Some((observer, error)) = trampoline.park(self, error) {
                    trampoline.run(|| observer.retry_once(error), Self::retry_once);
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

/// Where a retry waits while the operator is still subscribing.
///
/// The observer is handed back by `on_termination`, so an error that arrives while a `subscribe`
/// made by this operator has not returned yet — synchronously from that very call, or from another
/// thread meanwhile — parks the observer and its error here and returns. [`run`](Self::run) takes
/// them once the subscription returns and retries them itself. An error that arrives once no
/// subscription is being made finds the trampoline idle and retries on the spot. There is one
/// observer at a time, so at most one of them is ever parked.
#[derive(Educe)]
#[educe(Clone(bound()))]
struct Trampoline<M: ThreadMode, O, E>(M::Ptr<Round<O, E>>);

/// The state of a [`Trampoline`].
enum Round<O, E> {
    /// No subscription is being made.
    Idle,
    /// [`run`](Trampoline::run) is making subscriptions, and no error has arrived since it took
    /// the last one.
    Subscribing,
    /// An error arrived while a subscription was being made, and waits for it to return.
    Parked(O, E),
}

impl<M: ThreadMode, O, E> Default for Trampoline<M, O, E> {
    fn default() -> Self {
        Self(M::ptr(Round::Idle))
    }
}

impl<M: ThreadMode, O, E> Trampoline<M, O, E> {
    /// Runs `subscribe`, then `retry` on the error that arrived meanwhile, if any, and so on: a
    /// retry that fails while it is being subscribed parks again and is picked up by this loop, so
    /// the stack stays as deep as one subscription however many times in a row it fails.
    ///
    /// The trampoline stays subscribing from the first call to the last: between two of them, the
    /// observer is in this loop's hands, so no error can arrive. A parked observer holds this
    /// trampoline, which then holds the observer back: if a call unwinds, the guard empties the
    /// trampoline so that the two do not keep each other alive.
    fn run(&self, subscribe: impl FnOnce(), retry: impl Fn(O, E)) {
        let previous = self.0.replace_value(Round::Subscribing);
        debug_assert!(
            matches!(previous, Round::Idle),
            "one subscription is made at a time"
        );
        let _guard = on_panic(|| drop(self.0.replace_value(Round::Idle)));
        subscribe();
        while let Some((observer, error)) = self.take_parked() {
            retry(observer, error);
        }
    }

    /// Takes the observer and the error parked during the call that just returned, staying
    /// subscribing for their retry, or goes idle if none was parked.
    fn take_parked(&self) -> Option<(O, E)> {
        self.0
            .with_mut(|round| match std::mem::replace(round, Round::Idle) {
                Round::Parked(observer, error) => {
                    *round = Round::Subscribing;
                    Some((observer, error))
                }
                Round::Subscribing => None,
                Round::Idle => unreachable!("the trampoline was subscribing"),
            })
    }

    /// Parks `observer` and `error` while a subscription is being made, or hands them back to be
    /// retried at once.
    fn park(&self, observer: O, error: E) -> Option<(O, E)> {
        self.0.with_mut(|round| match round {
            Round::Subscribing => {
                *round = Round::Parked(observer, error);
                None
            }
            Round::Idle => Some((observer, error)),
            Round::Parked(..) => unreachable!("one observer at a time, so one error at a time"),
        })
    }
}
