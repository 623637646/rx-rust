//! Maps each item to an observable, ignoring the items that arrive while one is running.
//!
//! The [`ExhaustMap`] operator, behind
//! [`ObservableExt::exhaust_map`](crate::observable::ObservableExt::exhaust_map).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::combining::{
        exhaust_all::{self, ExhaustAllInnerObserver, ExhaustAllObserverOf},
        internal::flatten::{self, Ignore},
    },
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Maps each source value to an observable and emits its values, ignoring the source values that
/// arrive while the previous one is still running.
/// See <https://reactivex.io/documentation/operators/flatmap.html> (`exhaustMap`)
///
/// The callback is not called for an ignored value, so no work is started for it. See
/// [`ExhaustAll`](crate::operators::combining::exhaust_all::ExhaustAll) for the flattening this is built on.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::exhaust_map::ExhaustMap,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut clicks = PublishSubject::<_, Infallible, _>::local();
/// let mut response = PublishSubject::local();
///
/// let response_source = response.clone();
/// let subscription = ExhaustMap::new(clicks.clone(), move |click| {
///     response_source.clone().map(move |body| (click, body))
/// })
/// .subscribe_with_callback(|value| values.push(value), |_| {});
///
/// let _ = clicks.on_next(1);
/// let _ = clicks.on_next(2); // Ignored: the request of click 1 is still running.
/// let _ = response.on_next("done");
/// response.on_termination(Termination::Completed);
///
/// drop((subscription, clicks));
/// assert_eq!(values, [(1, "done")]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ExhaustMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> ExhaustMap<T0, OE, OE1, F> {
    /// Creates an [`ExhaustMap`] over `source`;
    /// [`ObservableExt::exhaust_map`](crate::observable::ObservableExt::exhaust_map) is the fluent
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for ExhaustMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`ExhaustAll`](crate::operators::combining::exhaust_all::ExhaustAll) this
    /// operator is, after a map.
    type Disposal = exhaust_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for ExhaustMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<ExhaustAllObserverOf<OE, OE1, T, E, OR, F>, Item = T0, Error = E>,
    OE1: Observable<ExhaustAllInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // Not a `Map` in front of an `ExhaustAll`: the callback is the selector of the machinery
        // both are built on, which decides when it runs.
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Ignore,
            self.callback,
        )
        .map_inner_into()
    }
}
