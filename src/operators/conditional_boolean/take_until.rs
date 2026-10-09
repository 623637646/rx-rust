//! The [`TakeUntil`] operator, behind
//! [`ObservableExt::take_until`](crate::observable::ObservableExt::take_until).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits the items emitted by a source Observable until a second Observable emits an item or
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
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let mut stop: PublishSubject<'_, (), Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = TakeUntil::new(source.clone(), stop.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// source.on_next(2);
/// stop.on_next(());
/// source.on_next(3);
/// drop(subscription);
///
/// assert_eq!(&*values.lock().unwrap(), &[1, 2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
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
    pub fn new<T, E>(source: OE, stop: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
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
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type Disposal = Disposal<Joined<OE::Mode, OE1::Mode>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for TakeUntil<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            TakeUntilObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            StopObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
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
