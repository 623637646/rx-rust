//! The [`FlatMap`] operator, behind
//! [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map).

use super::map::Map;
use crate::operators::combining::merge_all::MergeAll;
use crate::operators::combining::merge_all::MergeAllInnerObserver;
use crate::operators::combining::merge_all::MergeAllObserver;
use crate::operators::transforming::map::MapObserver;
use crate::thread_mode::Joined;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Maps each source value to an Observable and emits the values of all of them as they arrive.
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
/// let mut terminations = Vec::new();
///
/// let observable = FlatMap::new(FromIter::new(vec![1, 2]), |value| {
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
pub struct FlatMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> FlatMap<T0, OE, OE1, F> {
    /// Creates a [`FlatMap`] over `source`;
    /// [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map) is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T0, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
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
    type Mode = Joined<OE::Mode, OE1::Mode>;
    /// The disposal of the [`MergeAll`] this operator is built on.
    type Disposal = crate::operators::combining::merge_all::Disposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        OE::Disposal,
        OE1::Disposal,
    >;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for FlatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                MergeAllObserver<
                    Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
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
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
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
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observable = Map::new(self.source, self.callback);
        let observable = MergeAll::new(observable);
        observable.subscribe(observer)
    }
}
