//! Maps each item to an observable, and emits their items one observable after the other.
//!
//! The [`ConcatMap`] operator, behind
//! [`ObservableExt::concat_map`](crate::observable::ObservableExt::concat_map).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::combining::{
        internal::flatten::{self, Enqueue},
        merge_all::{self, MergeAllInnerObserver, MergeAllObserverOf},
    },
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Maps each source value to an observable and emits all of the values of each, one observable
/// after the other.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// It is [`FlatMap`](crate::operators::transforming::flat_map::FlatMap) with a
/// `max_concurrency` of one. The callback runs as each value arrives, and the observable it
/// returns waits in a queue, which has no bound, until the ones before it have completed: it is
/// subscribed only then, so an observable that starts its work as it is subscribed — most of
/// them — runs alone.
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
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) this
    /// operator is, after a map: that of a
    /// [`MergeAll`](crate::operators::combining::merge_all::MergeAll).
    type Disposal = merge_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for ConcatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<MergeAllObserverOf<OE, OE1, T, E, OR, F>, Item = T0, Error = E>,
    OE1: Observable<MergeAllInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // Not a `Map` in front of a `MergeAll`: the callback is the selector of the machinery
        // both are built on, which decides when it runs.
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Enqueue::new(),
            self.callback,
        )
        .map_inner_into()
    }
}
