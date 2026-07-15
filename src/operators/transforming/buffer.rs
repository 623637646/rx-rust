use crate::utils::subscribe_with_context::{
    self, Context, ModificationResult, subscribe_with_context,
};
use crate::utils::types::MaybeSend;
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal},
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::{
        self, subscribe_with_auto_dispose_on_termination,
    },
};
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
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, boundary }
    }
}

delegate_disposal!(
    Disposal<'or, D, D1>,
    subscribe_with_auto_dispose_on_termination::Disposal<subscribe_with_context::Disposal<'or, ChainDisposal<D, D1>>>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E, OE, OE1> Observable<'or, Vec<T>, E> for Buffer<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<'or, OE::D, OE1::D>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            subscribe_with_context(observer, Vec::new(), |context| {
                let subscription_1 = self.boundary.subscribe(BoundaryObserver(context.clone()));
                let subscription_2 = self.source.subscribe(BufferObserver(context));
                subscription_1.preceded_by_bound(subscription_2)
            })
        })
        .map_into()
    }
}

struct BufferObserver<T, E, OR>(Context<Vec<T>, E, OR, Vec<T>>);

impl<T, E, OR> Observer<T, E> for BufferObserver<T, E, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.modify_model(|values| {
            values.push(value);
            ModificationResult::new_empty()
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|values| {
                    if values.is_empty() {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        ModificationResult::new_send_next_and_termination(
                            std::mem::take(values),
                            termination,
                        )
                    }
                });
            }
            Termination::Error(_) => self.0.send_termination(termination),
        }
    }
}

struct BoundaryObserver<T, E, OR>(Context<Vec<T>, E, OR, Vec<T>>);

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, E, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        let _ = self.0.modify_model(|values| {
            ModificationResult::new_send_next(std::mem::replace(
                values,
                Vec::with_capacity(values.len()),
            ))
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|values| {
                    if values.is_empty() {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        ModificationResult::new_send_next_and_termination(
                            std::mem::take(values),
                            termination,
                        )
                    }
                });
            }
            Termination::Error(_) => self.0.send_termination(termination),
        }
    }
}
