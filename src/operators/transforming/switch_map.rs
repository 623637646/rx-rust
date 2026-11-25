//! Maps each item to an observable, and emits the items of the most recent one only.
//!
//! The [`SwitchMap`] operator, behind
//! [`ObservableExt::switch_map`](crate::observable::ObservableExt::switch_map).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::combining::{
        internal::flatten::{self, Replace},
        switch::{self, SwitchInnerObserver, SwitchObserverOf},
    },
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Maps each source value to an observable and emits the values of the most recent one only,
/// unsubscribing from the previous one.
/// See <https://reactivex.io/documentation/operators/switch.html>
///
/// The callback runs for every value, before the previous observable is unsubscribed from. See
/// [`Switch`](crate::operators::combining::switch::Switch) for the flattening this is
/// built on.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::switch_map::SwitchMap,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut queries = PublishSubject::<_, Infallible, _>::local();
/// let mut results = PublishSubject::local();
///
/// let results_source = results.clone();
/// let subscription = SwitchMap::new(queries.clone(), move |query| {
///     results_source.clone().map(move |result| (query, result))
/// })
/// .subscribe_with_callback(|value| values.push(value), |_| {});
///
/// let _ = queries.on_next("r");
/// let _ = queries.on_next("rx"); // The search for "r" is unsubscribed from.
/// let _ = results.on_next(3);
///
/// drop((subscription, queries, results));
/// assert_eq!(values, [("rx", 3)]);
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
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`Switch`](crate::operators::combining::switch::Switch) this
    /// operator shares its machinery with.
    type Disposal = switch::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for SwitchMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<SwitchObserverOf<OE, OE1, T, E, OR, F>, Item = T0, Error = E>,
    OE1: Observable<SwitchInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // Not a `Map` in front of a `Switch`: the callback is the selector of the machinery
        // both are built on, which decides when it runs.
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Replace,
            self.callback,
        )
        .map_inner_into()
    }
}
