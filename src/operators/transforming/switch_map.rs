//! The [`SwitchMap`] operator, behind
//! [`ObservableExt::switch_map`](crate::observable::ObservableExt::switch_map).

use super::map::Map;
use crate::operators::combining::switch::Switch;
use crate::operators::combining::switch::SwitchInnerObserver;
use crate::operators::combining::switch::SwitchObserver;
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

/// Maps each source value to an Observable and emits the values of the most recent one only,
/// unsubscribing from the previous one.
/// See <https://reactivex.io/documentation/operators/switch.html>
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
    /// Creates a [`SwitchMap`] over `source`;
    /// [`ObservableExt::switch_map`](crate::observable::ObservableExt::switch_map) is the fluent
    /// form.
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for SwitchMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    /// The disposal of the [`Switch`] this operator is built on.
    type D = crate::operators::combining::switch::Disposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        OE::D,
        OE1::D,
    >;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for SwitchMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                SwitchObserver<
                    Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                    T,
                    E,
                    OR,
                    <OE1 as ObservableTypes>::D,
                    <OE as ObservableTypes>::D,
                >,
                F,
            >,
            Item = T0,
            Error = E,
        >,
    OE1: Observable<
            SwitchInnerObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::D,
                <OE as ObservableTypes>::D,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observable = Map::new(self.source, self.callback);
        let observable = Switch::new(observable);
        observable.subscribe(observer)
    }
}
