//! The [`SwitchMap`] operator, behind
//! [`ObservableExt::switch_map`](crate::observable::ObservableExt::switch_map).

use super::map::Map;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::{
        combining::switch::{self, Switch, SwitchInnerObserver, SwitchObserver},
        transforming::map::MapObserver,
    },
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Maps each source value to an observable and emits the values of the most recent one only,
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
///
/// let observable = SwitchMap::new(FromIter::new(vec![1, 2]), |value| {
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
pub struct SwitchMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> SwitchMap<T0, OE, OE1, F> {
    /// Creates a [`SwitchMap`] over `source`;
    /// [`ObservableExt::switch_map`](crate::observable::ObservableExt::switch_map) is the fluent
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for SwitchMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`Switch`] this operator is built on.
    type Disposal = switch::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for SwitchMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            MapObserver<
                SwitchObserver<
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
            SwitchInnerObserver<
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
        let observable = Switch::new(observable);
        observable.subscribe(observer)
    }
}
