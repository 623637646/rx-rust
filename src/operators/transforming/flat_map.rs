//! The [`FlatMap`] operator, behind
//! [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map).

use super::map::Map;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::{
        combining::merge_all::{self, MergeAll, MergeAllInnerObserver, MergeAllObserver},
        transforming::map::MapObserver,
    },
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Maps each source value to an observable and emits the values of all of them as they arrive.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::flat_map::FlatMap,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = FlatMap::new(FromIter::new(vec![1, 2]), |value| {
///     FromIter::new(vec![value, value + 10])
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
pub struct FlatMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> FlatMap<T0, OE, OE1, F> {
    /// Creates a [`FlatMap`] over `source`;
    /// [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map) is the fluent form.
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for FlatMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`MergeAll`] this operator is built on.
    type Disposal = merge_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for FlatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                MergeAllObserver<
                    JoinedModeOf<OE, OE1>,
                    T,
                    E,
                    OR,
                    <OE1 as ObservableTypes>::Disposal,
                    <OE as ObservableTypes>::Disposal,
                >,
                F,
            >,
            Item = T0,
            Error = E,
        >,
    OE1: Observable<
            MergeAllInnerObserver<
                JoinedModeOf<OE, OE1>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::Disposal,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observable = Map::new(self.source, self.callback);
        let observable = MergeAll::new(observable);
        observable.subscribe(observer)
    }
}
