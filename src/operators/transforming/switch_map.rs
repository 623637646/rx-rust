use super::map::Map;
use crate::operators::combining::switch::{self, Switch};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable, observable::Subscription, observer::Observer, utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Projects each source value to an Observable which is merged in the output Observable, emitting values only from the most recently projected Observable.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T0, E>,
        OE1: Observable<'or, T, E>,
        F: FnMut(T0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, T0, T, E, OE, OE1, F> Observable<'or, T, E> for SwitchMap<T0, OE, OE1, F>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T0, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    F: FnMut(T0) -> OE1 + MaybeSend + 'or,
{
    type D = switch::Disposal<'or, OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observable = Map::new(self.source, self.callback);
        let observable = Switch::new(observable);
        observable.subscribe(observer)
    }
}
