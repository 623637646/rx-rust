use crate::utils::types::{MaybeSend, Mutable, Shared};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal},
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::{self, subscribe_unsub_after_termination},
};
use crate::{safe_lock, safe_lock_option_observer, safe_lock_vec};
use educe::Educe;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, when a `boundary` Observable emits an item.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::buffer::Buffer,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = Buffer::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// source.on_next(2);
/// boundary.on_next(());
/// source.on_next(3);
/// source.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(&*values.lock().unwrap(), &[vec![1, 2], vec![3]]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Buffer<OE, OE1> {
    pub fn new<'or, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
        OE1: Observable<'or, T = (), E = E>,
    {
        Self { source, boundary }
    }
}

delegate_disposal!(
    Disposal<D, D1>,
    subscribe_unsub_after_termination::Disposal<ChainDisposal<D, D1>>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E, OE, OE1> Observable<'or> for Buffer<OE, OE1>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T = T, E = E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T = (), E = E>,
    OE1::D: MaybeSend + 'or,
{
    type T = Vec<T>;
    type E = E;
    type D = Disposal<OE::D, OE1::D>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let values = Shared::new(Mutable::new(Vec::default()));
            let subscription_1 = self.boundary.subscribe(BoundaryObserver {
                observer: observer.clone(),
                values: values.clone(),
            });
            let subscription_2 = self.source.subscribe(BufferObserver { observer, values });
            subscription_1.preceded_by_bound(subscription_2)
        })
        .map_into()
    }
}

struct BufferObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_vec!(push: self.values, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let values = safe_lock!(mem_take: self.values);
                if !values.is_empty() {
                    safe_lock_option_observer!(on_next_and_termination: self.observer, values, termination);
                } else {
                    safe_lock_option_observer!(on_termination: self.observer, termination);
                }
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}

struct BoundaryObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        let values = safe_lock!(mem_take: self.values);
        safe_lock_option_observer!(on_next: self.observer, values);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let values = safe_lock!(mem_take: self.values);
                if !values.is_empty() {
                    safe_lock_option_observer!(on_next_and_termination: self.observer, values, termination);
                } else {
                    safe_lock_option_observer!(on_termination: self.observer, termination);
                }
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
