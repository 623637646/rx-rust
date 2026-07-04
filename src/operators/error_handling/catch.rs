use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
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
///     observable::observable_ext::ObservableExt,
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
/// let observable = Catch::new(Throw::new("boom").map_infallible_to_value(), |error| Just::new(error));
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
    pub fn new<'or, 'sub, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E0>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnOnce(E0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E0, E, OE, OE1, F> Observable<'or, 'sub, T, E> for Catch<E0, OE, F>
where
    E: 'or,
    OE: Observable<'or, 'sub, T, E0>,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnOnce(E0) -> OE1 + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let shared_disposal = SharedDisposal::default();
        let observer = CatchObserver {
            observer,
            callback: self.callback,
            shared_disposal: shared_disposal.clone(),
            _marker: PhantomData,
        };
        self.source.subscribe(observer) + shared_disposal
    }
}

struct CatchObserver<'sub, E, OR, F> {
    observer: OR,
    callback: F,
    shared_disposal: SharedDisposal<Subscription<'sub>>,
    _marker: MarkerType<E>,
}

impl<'or, 'sub, T, E0, E, OR, OE1, F> Observer<T, E0> for CatchObserver<'sub, E, OR, F>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnOnce(E0) -> OE1,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
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
