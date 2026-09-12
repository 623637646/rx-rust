use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
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
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T0, E>,
        F: FnMut(T0) -> T,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, T0, T, E, OE, F> Observable<'or, T, E> for Map<T0, OE, F>
where
    OE: Observable<'or, T0, E>,
    F: FnMut(T0) -> T + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = MapObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct MapObserver<OR, F> {
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
