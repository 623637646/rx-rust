use crate::delegate_disposal;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal,
};
use crate::observable::Subscription;
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Catches errors on the observable to be handled by returning a new observable or throwing an error.
/// See <https://reactivex.io/documentation/operators/catch.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{just::Just, throw::Throw},
///         error_handling::catch::Catch,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Catch::new(Throw::new("boom").with_item_type(), |error| Just::new(error));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec!["boom"]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Catch<E0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<E0>,
}

impl<E0, OE, F> Catch<E0, OE, F> {
    pub fn new<'or, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E0>,
        OE1: Observable<'or, T, E>,
        F: FnOnce(E0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<D, D1>,
    ChainDisposal<SharedDisposal<Subscription<D1>>, D>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E0, E, OE, OE1, F> Observable<'or, T, E> for Catch<E0, OE, F>
where
    E: 'or,
    OE: Observable<'or, T, E0>,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    F: FnOnce(E0) -> OE1 + MaybeSend + 'or,
{
    type D = Disposal<OE::D, OE1::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let shared_disposal = SharedDisposal::default();
        let observer = CatchObserver {
            observer,
            callback: self.callback,
            shared_disposal: shared_disposal.clone(),
            _marker: PhantomData,
        };
        self.source
            .subscribe(observer)
            .preceded_by(shared_disposal)
            .map_into()
    }
}

struct CatchObserver<E, OR, F, D: Disposable> {
    observer: OR,
    callback: F,
    shared_disposal: SharedDisposal<Subscription<D>>,
    _marker: MarkerType<E>,
}

impl<'or, T, E0, E, OR, OE1, F> Observer<T, E0> for CatchObserver<E, OR, F, OE1::D>
where
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    F: FnOnce(E0) -> OE1,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E0>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => {
                self.shared_disposal.replace(|| {
                    let observable = (self.callback)(error);
                    observable.subscribe(self.observer)
                });
            }
        }
    }
}
