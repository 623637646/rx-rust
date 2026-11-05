//! The [`RepeatSource`] operator, behind
//! [`ObservableExt::repeat_source`](crate::observable::ObservableExt::repeat_source).

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

/// Re-subscribes after a completion, to the observable the callback returns, until the callback
/// returns `None`, which completes downstream. An error is not repeated: it goes downstream at
/// once.
/// See <https://reactivex.io/documentation/operators/repeat.html>
///
/// This is the operator form of ReactiveX's `Repeat`, which subscribes to a source again;
/// [`Repeat`](crate::operators::creating::repeat::Repeat) is the source that emits one item a
/// number of times. The callback builds each round's observable, so the source need not be
/// `Clone`, and a round may subscribe to a different observable than the one before.
///
/// Once the subscription is disposed, the callback is no longer called: a completion that arrives
/// after that is dropped with the observer, which is not terminated.
///
/// An observable that completes while it is being subscribed is repeated once its `subscribe` has
/// returned, not from inside it: however many times in a row a source completes synchronously, the
/// rounds take the stack of one subscription. The subscription of the observable that completed is
/// disposed before the next one is subscribed.
///
/// A completion and the subscription before it cross threads in the `Shared` mode. A completion
/// that arrives on another thread while a subscription is being made waits for it to return, and
/// is repeated by the thread that made it; otherwise it is repeated on the thread it arrived on.
/// Each round disposes the subscription before it, the source's included, on the thread that runs
/// the round. So behind a source that requires a `Send` observer, the source's disposal must be
/// `Send`; the `Local` mode does not require it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::from_iter::FromIter, utility::repeat_source::RepeatSource},
/// };
///
/// let mut values = Vec::new();
/// let mut rounds = 1;
///
/// let observable = RepeatSource::new(FromIter::new([1, 2]), move || {
///     rounds += 1;
///     (rounds <= 3).then(|| FromIter::new([1, 2]))
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 1, 2, 1, 2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct RepeatSource<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> RepeatSource<OE, F> {
    /// Creates a [`RepeatSource`] over `source`;
    /// [`ObservableExt::repeat_source`](crate::observable::ObservableExt::repeat_source) is the
    /// fluent form.
    pub fn new<T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
        F: FnMut() -> Option<OE1>,
    {
        Self { source, callback }
    }
}

/// The subscription of one round: of the source (`D`), of an observable the callback returned
/// (`D1`), or nothing once the callback returned `None`.
type RoundDisposal<D, D1> = OptionDisposal<EitherDisposal<D, D1>>;

delegate_disposal!(
    Disposal<M, D, D1>,
    ReplaceableDisposal<M, RoundDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1, F> ObservableTypes for RepeatSource<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut() -> Option<OE1>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, F, OR> Observable<OR> for RepeatSource<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            RepeatSourceObserver<
                JoinedModeOf<OE, OE1>,
                OR,
                F,
                <OE as ObservableTypes>::Disposal,
                OE1,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            RepeatSourceObserver<
                JoinedModeOf<OE, OE1>,
                OR,
                F,
                <OE as ObservableTypes>::Disposal,
                OE1,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut() -> Option<OE1>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let replaceable_disposal = ReplaceableDisposal::default();
        let trampoline = Trampoline::default();
        let observer = RepeatSourceObserver {
            observer,
            callback: self.callback,
            replaceable_disposal: replaceable_disposal.clone(),
            resubscribe: SubscribeFn::new(),
            trampoline: trampoline.clone(),
        };
        // The source's subscription goes into the same slot as the later rounds', so that the
        // second round disposes it before subscribing, as each round disposes the one before.
        trampoline.run(
            || {
                replaceable_disposal.replace_with(|| {
                    self.source
                        .subscribe(observer)
                        .map_inner(|disposal| OptionDisposal::some(EitherDisposal::Left(disposal)))
                })
            },
            RepeatSourceObserver::repeat_once,
        );
        DisposeOnDrop::new(replaceable_disposal).map_inner_into()
    }
}

#[doc(hidden)]
pub struct RepeatSourceObserver<M: ThreadMode, OR, F, D: Disposable, OE1: ObservableTypes> {
    observer: OR,
    callback: F,
    /// Holds the subscription of the source, then of each observable the callback returned, or
    /// nothing once the callback returned `None`.
    replaceable_disposal: ReplaceableDisposal<M, RoundDisposal<D, OE1::Disposal>>,
    /// Subscribes the observable the callback returned, with this observer.
    resubscribe: SubscribeFn<OE1, Self>,
    /// Where a completion that arrives while this operator is subscribing waits for the
    /// subscription to return, so that rounds follow one another instead of nesting.
    trampoline: Trampoline<M, Self>,
}

impl<M, T, E, OR, D, OE1, F> Observer<T, E> for RepeatSourceObserver<M, OR, F, D, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut() -> Option<OE1>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let trampoline = self.trampoline.clone();
                if let Some(observer) = trampoline.park(self) {
                    trampoline.run(|| observer.repeat_once(), Self::repeat_once);
                }
            }
            error @ Termination::Error(_) => self.observer.on_termination(error),
        }
    }
}

impl<M, T, E, OR, D, OE1, F> RepeatSourceObserver<M, OR, F, D, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut() -> Option<OE1>,
{
    /// Asks the callback for the next round, and subscribes to the observable it returns or
    /// completes downstream.
    ///
    /// The callback is asked from the builder, which does not run once the subscription is
    /// disposed: a completion that arrives then — from the source's own disposal, say — is dropped
    /// with the observer, without calling the callback.
    fn repeat_once(mut self) {
        self.replaceable_disposal
            .clone()
            .replace_with(|| match (self.callback)() {
                Some(observable) => {
                    let resubscribe = self.resubscribe;
                    resubscribe
                        .subscribe(observable, self)
                        .map_inner(|disposal| OptionDisposal::some(EitherDisposal::Right(disposal)))
                }
                None => {
                    self.observer.on_termination(Termination::Completed);
                    DisposeOnDrop::new(OptionDisposal::none())
                }
            })
    }
}
