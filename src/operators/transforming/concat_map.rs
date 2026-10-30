//! The [`ConcatMap`] operator, behind
//! [`ObservableExt::concat_map`](crate::observable::ObservableExt::concat_map).

use super::map::Map;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::{
        combining::concat_all::{self, ConcatAll, InnerObserver, SourceObserver},
        transforming::map::MapObserver,
    },
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Maps each source value to an observable and emits all of the values of each, one observable
/// after the other.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::concat_map::ConcatMap,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = ConcatMap::new(FromIter::new([1, 2]), |value| {
///     FromIter::new([value, value + 10])
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 11, 2, 12]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> ConcatMap<T0, OE, OE1, F> {
    /// Creates a [`ConcatMap`] over `source`;
    /// [`ObservableExt::concat_map`](crate::observable::ObservableExt::concat_map) is the fluent
    /// form.
    pub fn new<E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T0, Error = E>,
        OE1: ObservableTypes<Error = E>,
        F: FnMut(T0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<T0, T, E, OE, OE1, F> ObservableTypes for ConcatMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`ConcatAll`] this operator is built on.
    type Disposal = concat_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for ConcatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                SourceObserver<
                    JoinedModeOf<OE, OE1>,
                    T,
                    E,
                    OR,
                    OE1,
                    <OE as ObservableTypes>::Disposal,
                >,
                F,
            >,
            Item = T0,
            Error = E,
        >,
    OE1: Observable<
            InnerObserver<JoinedModeOf<OE, OE1>, T, E, OR, OE1, <OE as ObservableTypes>::Disposal>,
            Item = T,
            Error = E,
        >,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observable = Map::new(self.source, self.callback);
        let observable = ConcatAll::new(observable);
        observable.subscribe(observer)
    }
}
