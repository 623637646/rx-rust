use crate::delegate_disposal;
use crate::disposable::Disposable;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::safe_lock_option_observer;
use crate::utils::subscribe_unsub_after_termination;
use crate::utils::types::{MaybeSend, Mutable, Shared};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::{
        subscribe_unsub_after_termination::subscribe_unsub_after_termination, types::MarkerType,
    },
};
use educe::Educe;
use std::marker::PhantomData;

/// Emits the items emitted by a source Observable until a second Observable emits an item or a notification.
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
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut stop: PublishSubject<'_, (), Infallible> = PublishSubject::default();
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
    pub fn new<'or, T, E>(source: OE, stop: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, stop }
    }
}

delegate_disposal!(
    Disposal<D, D1>,
    subscribe_unsub_after_termination::Disposal<ChainDisposal<D, D1>>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for TakeUntil<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<OE::D, OE1::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let subscription_1 = self.stop.subscribe(StopObserver {
                observer: observer.clone(),
                _marker: PhantomData,
            });
            let subscription_2 = self.source.subscribe(TakeUntilObserver(observer));
            subscription_1.preceded_by_bound(subscription_2)
        })
        .map_into()
    }
}

struct TakeUntilObserver<OR>(Shared<Mutable<Option<OR>>>);

impl<T, E, OR> Observer<T, E> for TakeUntilObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_option_observer!(on_next: self.0, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.0, termination);
    }
}

struct StopObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<(), E> for StopObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        safe_lock_option_observer!(on_termination: self.observer, Termination::Completed);
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}
