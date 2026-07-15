use crate::delegate_disposal;
use crate::utils::subscribe_with_auto_dispose_on_termination;
use crate::utils::subscribe_with_context::{self, Context, subscribe_with_context};
use crate::utils::types::MaybeSend;
use crate::{
    disposable::{Disposable, chain_disposal::ChainDisposal},
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

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
    Disposal<'or, D, D1>,
    subscribe_with_auto_dispose_on_termination::Disposal<subscribe_with_context::Disposal<'or, ChainDisposal<D, D1>>>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for TakeUntil<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<'or, OE::D, OE1::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            subscribe_with_context(observer, (), |context| {
                let subscription_1 = self.stop.subscribe(StopObserver(context.clone()));
                let subscription_2 = self.source.subscribe(TakeUntilObserver(context));
                subscription_1.preceded_by_bound(subscription_2)
            })
        })
        .map_into()
    }
}

struct TakeUntilObserver<T, E, OR>(Context<T, E, OR, ()>);

impl<T, E, OR> Observer<T, E> for TakeUntilObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

struct StopObserver<T, E, OR>(Context<T, E, OR, ()>);

impl<T, E, OR> Observer<(), E> for StopObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        self.0.send_termination(Termination::Completed);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}
