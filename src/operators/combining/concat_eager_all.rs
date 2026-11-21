//! The [`ConcatEagerAll`] operator, behind
//! [`ObservableExt::concat_eager_all`](crate::observable::ObservableExt::concat_eager_all).

use super::internal::{
    flatten::ContextDisposal,
    reorder::{InnerObserverOf, SourceObserverOf, subscribe_eager},
};
pub use super::internal::{
    flatten::{Enqueue, Identity, InnerObserver, SourceObserver},
    reorder::{IndexedEvent, IndexedSource, IndexingObserver, IndexingSelector, ReorderObserver},
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    thread_mode::ThreadMode,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Flattens an observable of observables by subscribing each inner observable as it arrives, and
/// emitting all of the values of each, one inner observable after the other.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// The order is that of [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll), the
/// subscriptions those of [`MergeAll`](crate::operators::combining::merge_all::MergeAll): the
/// inner observables run side by side, and the values of the ones behind the first are kept until
/// every one before them has completed. So the work is done ahead of its turn, at the price of
/// those values, which are kept without a bound: for a few requests whose answers are wanted in
/// the order they were asked in, not for inner observables that never end.
///
/// [`ConcatEagerAll::max_concurrency`] subscribes at most `max_concurrency` of them at a time,
/// the first included: one that arrives while that many are running waits in a queue, and is
/// subscribed once one of them completes. With a `max_concurrency` of one it is `ConcatAll`.
///
/// The stream completes once the source and every inner observable have completed. An error of
/// the source or of any inner observable that is subscribed ends it at once, ahead of the values
/// the ones before it have yet to send, and drops those that were kept.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::concat_eager_all::ConcatEagerAll,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::<_, Infallible, _>::local();
/// let mut inner_1 = PublishSubject::local();
/// let mut inner_2 = PublishSubject::local();
///
/// let subscription = ConcatEagerAll::new(source.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(inner_1.clone());
/// let _ = source.on_next(inner_2.clone()); // Subscribed at once, unlike with `ConcatAll`.
/// let _ = inner_2.on_next(3); // Kept: `inner_1` has not completed.
/// let _ = inner_1.on_next(1);
/// let _ = inner_1.on_next(2);
/// inner_1.on_termination(Termination::Completed); // What `inner_2` sent is emitted.
/// let _ = inner_2.on_next(4);
///
/// drop((subscription, source, inner_2));
/// assert_eq!(values, [1, 2, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatEagerAll<OE, OE1> {
    source: OE,
    max_concurrency: NonZeroUsize,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatEagerAll<OE, OE1> {
    /// Creates a [`ConcatEagerAll`] over `source`, with no limit;
    /// [`ObservableExt::concat_eager_all`](crate::observable::ObservableExt::concat_eager_all) is
    /// the fluent form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            // No more than `usize::MAX` subscriptions can be held anyway, so nothing is ever
            // queued.
            max_concurrency: NonZeroUsize::MAX,
            _marker: PhantomData,
        }
    }

    /// Subscribes at most `max_concurrency` inner observables at a time: one that arrives while
    /// that many are running waits in a queue.
    pub fn max_concurrency(mut self, max_concurrency: NonZeroUsize) -> Self {
        self.max_concurrency = max_concurrency;
        self
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, OE1>,
    ContextDisposal<
        M,
        IndexedEvent<T>,
        E,
        D,
        Enqueue<IndexedSource<OE1>>,
        <OE1 as ObservableTypes>::Disposal,
    >,
    where M: ThreadMode, D: Disposable, OE1: ObservableTypes
);

impl<T, E, OE, OE1> ObservableTypes for ConcatEagerAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for ConcatEagerAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<SourceObserverOf<OE, OE1, T, E, OR, Identity>, Item = OE1, Error = E>,
    OE1: Observable<InnerObserverOf<OE, OE1, T, E, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_eager(self.source, Identity, observer, self.max_concurrency).map_inner_into()
    }
}
