use super::map::Map;
use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Projects each source value to an Observable which is merged in the output Observable, emitting values only from the most recently projected Observable.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::switch_map::SwitchMap,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = SwitchMap::new(FromIter::new(vec![1, 2]), |value| {
///     FromIter::new(vec![value, value + 10])
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 11, 2, 12]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SwitchMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> SwitchMap<T0, OE, OE1, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T0, E>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T0, T, E, OE, OE1, F> Observable<'or, 'sub, T, E> for SwitchMap<T0, OE, OE1, F>
where
    T: 'or,
    OE: Observable<'or, 'sub, T0, E>,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnMut(T0) -> OE1 + NecessarySendSync + 'or,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let observable = Map::new(self.source, self.callback);
        let observable = observable.switch();
        observable.subscribe(observer)
    }
}
