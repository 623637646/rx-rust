//! The [`TakeUntil`] operator, behind
//! [`ObservableExt::take_until`](crate::observable::ObservableExt::take_until).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
};
use educe::Educe;

/// Emits the items emitted by a source observable until a second observable emits an item or
/// terminates.
/// See <https://reactivex.io/documentation/operators/takeuntil.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::conditional_boolean::take_until::TakeUntil,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::<i32, Infallible, _>::local();
/// let mut stop = PublishSubject::<(), Infallible, _>::local();
///
/// let subscription = TakeUntil::new(source.clone(), stop.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// let _ = stop.on_next(()); // Completes the result.
/// let _ = source.on_next(3);
///
/// drop((subscription, source, stop));
/// assert_eq!(values, [1, 2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TakeUntil<OE, OE1> {
    source: OE,
    stop: OE1,
}

impl<OE, OE1> TakeUntil<OE, OE1> {
    /// Creates a [`TakeUntil`] over `source`;
    /// [`ObservableExt::take_until`](crate::observable::ObservableExt::take_until) is the fluent
    /// form.
    pub fn new<E>(source: OE, stop: OE1) -> Self
    where
        OE: ObservableTypes<Error = E>,
        OE1: ObservableTypes<Item = (), Error = E>,
    {
        Self { source, stop }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, T, E, (), ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for TakeUntil<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = (), Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for TakeUntil<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            TakeUntilObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            StopObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = (),
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_context(observer, (), |context| {
            let subscription_1 = self.stop.subscribe(StopObserver(context.clone()));
            let subscription_2 = self.source.subscribe(TakeUntilObserver(context));
            subscription_1.preceded_by_wrapped(subscription_2)
        })
        .map_inner_into()
    }
}

#[doc(hidden)]
pub struct TakeUntilObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, (), D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<T, E> for TakeUntilObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

#[doc(hidden)]
pub struct StopObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, (), D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<(), E> for StopObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) -> Flow {
        self.0.send_termination(Termination::Completed);
        // The first notification ends the stream, so the notifier is of no use afterwards.
        Flow::Stop
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}
