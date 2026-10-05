//! The [`ConcatMap`] operator, behind
//! [`ObservableExt::concat_map`](crate::observable::ObservableExt::concat_map).

use super::map::Map;
use crate::operators::combining::concat_all::ConcatAll;
use crate::operators::combining::concat_all::InnerObserver;
use crate::operators::combining::concat_all::SourceObserver;
use crate::operators::transforming::map::MapObserver;
use crate::thread_mode::Joined;
use crate::utils::subscribe_with_context;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Projects each source value to an Observable which is merged in a serialized fashion in the output Observable.
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
/// let mut terminations = Vec::new();
///
/// let observable = ConcatMap::new(FromIter::new(vec![1, 2]), |value| {
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
pub struct ConcatMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> ConcatMap<T0, OE, OE1, F> {
    /// Creates a [`ConcatMap`] over `source`;
    /// [`ObservableExt::concat_map`](crate::observable::ObservableExt::concat_map) is the fluent form.
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for ConcatMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type D = subscribe_with_context::ContextDisposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        crate::operators::combining::concat_all::Model<OE1>,
        OE::D,
    >;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for ConcatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                SourceObserver<
                    Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                    T,
                    E,
                    OR,
                    OE1,
                    <OE as ObservableTypes>::D,
                >,
                F,
            >,
            Item = T0,
            Error = E,
        >,
    OE1: Observable<
            InnerObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                OE1,
                <OE as ObservableTypes>::D,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observable = Map::new(self.source, self.callback);
        let observable = ConcatAll::new(observable);
        observable.subscribe(observer)
    }
}
