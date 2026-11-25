//! Maps each item to an observable, and emits their items as they arrive.
//!
//! The [`FlatMap`] operator, behind
//! [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map).

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

/// Maps each source value to an observable and emits the values of all of them as they arrive.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// [`FlatMap::new`] subscribes every observable the callback returns at once;
/// [`FlatMap::max_concurrency`] subscribes at most `max_concurrency` of them at a time.
///
/// The callback runs as each value arrives; with a limit, the observable it returns waits in a
/// queue until a place is free, and is subscribed only then, as in the
/// [`MergeAll`](crate::operators::combining::merge_all::MergeAll) this shares its machinery with.
/// So an observable that starts its work as it is subscribed — most of them — is bounded by
/// `max_concurrency`, while one that starts it as it is created, such as a future already running,
/// is not.
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
/// let observable = FlatMap::new(FromIter::new([1, 2]), |value| {
///     FromIter::new([value, value + 10])
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 11, 2, 12]);
/// ```
///
/// With a limit:
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::flat_map::FlatMap,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
///
/// let observable = FlatMap::new(FromIter::new([1, 2]), |value| {
///     FromIter::new([value, value + 10])
/// })
/// .max_concurrency(NonZeroUsize::new(1).unwrap());
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
    max_concurrency: NonZeroUsize,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> FlatMap<T0, OE, OE1, F> {
    /// Creates a [`FlatMap`] over `source`, with no limit;
    /// [`ObservableExt::flat_map`](crate::observable::ObservableExt::flat_map) is the fluent form.
    pub fn new<E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T0, Error = E>,
        OE1: ObservableTypes<Error = E>,
        F: FnMut(T0) -> OE1,
    {
        Self {
            source,
            // No more than `usize::MAX` subscriptions can be held anyway, so nothing is ever
            // queued.
            max_concurrency: NonZeroUsize::MAX,
            callback,
            _marker: PhantomData,
        }
    }

    /// Subscribes at most `max_concurrency` of the observables the callback returns at a time:
    /// one made while that many are running waits in a queue.
    pub fn max_concurrency(mut self, max_concurrency: NonZeroUsize) -> Self {
        self.max_concurrency = max_concurrency;
        self
    }
}

impl<T0, T, E, OE, OE1, F> ObservableTypes for FlatMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`MergeAll`](crate::operators::combining::merge_all::MergeAll) this
    /// operator shares its machinery with.
    type Disposal = merge_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for FlatMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<MergeAllObserverOf<OE, OE1, T, E, OR, F>, Item = T0, Error = E>,
    OE1: Observable<MergeAllInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // Not a `Map` in front of a `MergeAll`: the callback is the selector of the machinery
        // both are built on, which decides when it runs.
        flatten::subscribe(
            self.source,
            |_| observer,
            self.max_concurrency,
            Enqueue::new(),
            self.callback,
        )
        .map_inner_into()
    }
}
