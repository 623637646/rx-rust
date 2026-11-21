//! The [`ConcatEagerMap`] operator, behind
//! [`ObservableExt::concat_eager_map`](crate::observable::ObservableExt::concat_eager_map).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::combining::{
        concat_eager_all,
        internal::reorder::{InnerObserverOf, SourceObserverOf, subscribe_eager},
    },
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Maps each source value to an observable, subscribes it at once, and emits all of the values of
/// each, one observable after the other.
/// See <https://reactivex.io/documentation/operators/flatmap.html>
///
/// The order is that of [`ConcatMap`](crate::operators::transforming::concat_map::ConcatMap), the
/// subscriptions those of [`FlatMap`](crate::operators::transforming::flat_map::FlatMap): the
/// observables run side by side, and the values of the ones behind the first are kept, without a
/// bound, until every one before them has completed. So requests run in parallel and answer in
/// the order they were asked in. See
/// [`ConcatEagerAll`](crate::operators::combining::concat_eager_all::ConcatEagerAll) for the
/// flattening this is built on.
///
/// [`ConcatEagerMap::max_concurrency`] subscribes at most `max_concurrency` of them at a time.
/// The callback runs as each value arrives, whatever the limit.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::concat_eager_map::ConcatEagerMap,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut ids = PublishSubject::<_, Infallible, _>::local();
/// let mut slow = PublishSubject::<_, Infallible, _>::local();
/// let mut fast = PublishSubject::local();
///
/// let responses = [slow.clone(), fast.clone()];
/// let subscription = ConcatEagerMap::new(ids.clone(), move |id: usize| {
///     responses[id].clone().map(move |body: &str| (id, body))
/// })
/// .subscribe_with_callback(|value| values.push(value), |_| {});
///
/// let _ = ids.on_next(0);
/// let _ = ids.on_next(1); // Both requests run.
/// let _ = fast.on_next("second");
/// fast.on_termination(Termination::Completed); // Kept: the first has not answered.
/// let _ = slow.on_next("first");
/// slow.on_termination(Termination::Completed);
///
/// drop((subscription, ids));
/// assert_eq!(values, [(0, "first"), (1, "second")]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatEagerMap<T0, OE, OE1, F> {
    source: OE,
    max_concurrency: NonZeroUsize,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> ConcatEagerMap<T0, OE, OE1, F> {
    /// Creates a [`ConcatEagerMap`] over `source`, with no limit;
    /// [`ObservableExt::concat_eager_map`](crate::observable::ObservableExt::concat_eager_map) is
    /// the fluent form.
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

impl<T0, T, E, OE, OE1, F> ObservableTypes for ConcatEagerMap<T0, OE, OE1, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the
    /// [`ConcatEagerAll`](crate::operators::combining::concat_eager_all::ConcatEagerAll) this
    /// operator is built on.
    type Disposal = concat_eager_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T0, T, E, OE, OE1, F, OR> Observable<OR> for ConcatEagerMap<T0, OE, OE1, F>
where
    OR: Observer<T, E>,
    OE: Observable<SourceObserverOf<OE, OE1, T, E, OR, F>, Item = T0, Error = E>,
    OE1: Observable<InnerObserverOf<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: FnMut(T0) -> OE1,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // A `ConcatEagerAll` whose selector is the callback: every value is kept, so the callback
        // runs as each arrives, and the position of an inner observable is that of its value.
        subscribe_eager(self.source, self.callback, observer, self.max_concurrency).map_inner_into()
    }
}
