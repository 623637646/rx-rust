//! The [`Map`] operator, behind [`ObservableExt::map`](crate::observable::ObservableExt::map).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Transforms items emitted by an Observable by applying a function to each item.
/// See <https://reactivex.io/documentation/operators/map.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::map::Map,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Map::new(FromIter::new(vec![1, 2]), |value| value * 10);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![10, 20]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Map<T0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T0>,
}

impl<T0, OE, F> Map<T0, OE, F> {
    /// Creates a [`Map`] over `source`;
    /// [`ObservableExt::map`](crate::observable::ObservableExt::map) is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T0, Error = E>,
        F: FnMut(T0) -> T,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<T0, T, E, OE, F> ObservableTypes for Map<T0, OE, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    F: FnMut(T0) -> T,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<T0, T, E, OE, F, OR> Observable<OR> for Map<T0, OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<MapObserver<OR, F>, Item = T0, Error = E>,
    F: FnMut(T0) -> T,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observer = MapObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct MapObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T0, T, E, OR, F> Observer<T0, E> for MapObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(T0) -> T,
{
    fn on_next(&mut self, value: T0) -> Flow {
        self.observer.on_next((self.callback)(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
