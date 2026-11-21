//! The [`ConcatEager`] operator, behind
//! [`ObservableExt::concat_eager`](crate::observable::ObservableExt::concat_eager).

use super::internal::{
    merge::{ContextDisposal, DriverOf, Pause, subscribe_limited, subscribe_unlimited},
    reorder::IndexSourcesOps,
};
pub use super::internal::{
    merge::{MergeObserver, Unlimited},
    reorder::{
        IndexSources, IndexedEvent, IndexedSource, IndexedSourcesIter, IndexingObserver,
        ReorderObserver,
    },
};
use crate::{
    delegate_disposal,
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{
        IterSources, Observable, ObservableTypes, SameItemSources, SourceSet,
        sources::SubscribeEach,
    },
    observer::Observer,
    thread_mode::ThreadMode,
};
use educe::Educe;
use std::{convert::Infallible, num::NonZeroUsize};

/// Subscribes all of the observables at once, and emits all of the values of the first, then, once
/// it completes, all of the values of the next, and so on to the last.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// The order is that of [`Concat`](super::concat::Concat), the subscriptions those of
/// [`Merge`](super::merge::Merge): the sources run side by side, and the values of the ones
/// behind the first are kept until every one before them has completed. So the work is done ahead
/// of its turn, at the price of those values, which are kept without a bound: for a few requests
/// whose answers are wanted in the order they were asked in, not for sources that never end.
///
/// The sources are a tuple of one to eight observables of one item type and one error type, which
/// may be observables of different types: no boxing is needed to concatenate them.
/// [`ObservableExt::concat_eager`](crate::observable::ObservableExt::concat_eager) is the form for
/// two; [`ConcatEager::new_from_iter`] is the one for any number of observables of one type,
/// decided as the program runs. An observable of observables, which pushes them rather than being
/// pulled, goes to [`ConcatEagerAll`](super::concat_eager_all::ConcatEagerAll).
///
/// [`ConcatEager::max_concurrency`] subscribes at most `max_concurrency` of the sources at a
/// time, in order, the first included: the next one is pulled — from the tuple, or from the
/// iterator — only once one of them has completed, as [`Merge::max_concurrency`] does. With no
/// limit an iterator is pulled to its end as the stream is subscribed, so one that never ends
/// needs a limit. With a `max_concurrency` of one it is [`Concat`](super::concat::Concat).
///
/// The stream completes once every source has completed. An error of any source that is
/// subscribed ends it at once, ahead of the values the ones before it have yet to send, and drops
/// those that were kept.
///
/// [`Merge::max_concurrency`]: super::merge::Merge::max_concurrency
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::concat_eager::ConcatEager,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut first = PublishSubject::<_, Infallible, _>::local();
/// let mut second = PublishSubject::local();
///
/// let subscription = ConcatEager::new((first.clone(), second.clone().map(|value| value * 10)))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
///
/// let _ = second.on_next(3); // Subscribed at once, unlike with `Concat`, and kept.
/// let _ = first.on_next(1);
/// first.on_termination(Termination::Completed); // What `second` sent is emitted.
/// let _ = second.on_next(4);
///
/// drop((subscription, second));
/// assert_eq!(values, [1, 30, 40]);
/// ```
///
/// A list, with at most two of them running at a time:
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{combining::concat_eager::ConcatEager, creating::from_iter::FromIter},
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
///
/// let observable =
///     ConcatEager::new_from_iter((0..3).map(|index| FromIter::new([index * 10, index * 10 + 1])))
///         .max_concurrency(NonZeroUsize::new(2).unwrap());
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![0, 1, 10, 11, 20, 21]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatEager<S, L = Unlimited> {
    sources: S,
    limit: L,
}

impl<S> ConcatEager<S> {
    /// Creates a [`ConcatEager`] over the tuple `sources`, with no limit;
    /// [`ObservableExt::concat_eager`](crate::observable::ObservableExt::concat_eager) is the
    /// fluent form for two.
    pub fn new(sources: S) -> Self
    where
        S: SameItemSources,
    {
        Self {
            sources,
            limit: Unlimited,
        }
    }

    /// Subscribes at most `max_concurrency` of the sources at a time, the first included: the
    /// next one is pulled, from the tuple or from the iterator, only once one of them has
    /// completed, so an iterator that never ends can be concatenated. With a `max_concurrency` of
    /// one the sources run one after the other.
    pub fn max_concurrency(self, max_concurrency: NonZeroUsize) -> ConcatEager<S, NonZeroUsize> {
        ConcatEager {
            sources: self.sources,
            limit: max_concurrency,
        }
    }
}

impl<I> ConcatEager<IterSources<I>> {
    /// Concatenates the observables of an iterator, subscribing each as it is pulled: all of
    /// them at once, unless [`ConcatEager::max_concurrency`] sets a limit.
    ///
    /// Stops pulling sources when the stream stops; an empty iterator completes immediately.
    pub fn new_from_iter<OE>(sources: I) -> Self
    where
        I: IntoIterator<Item = OE>,
        OE: ObservableTypes,
    {
        Self {
            sources: IterSources(sources),
            limit: Unlimited,
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, C, SL>,
    ContextDisposal<M, IndexedEvent<T>, E, C, SL>,
    where M: ThreadMode
);

impl<T, S> ObservableTypes for ConcatEager<S>
where
    S: SameItemSources<SameItem = T>,
{
    type Item = T;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources still subscribed are disposed the last subscribed first. No source waits to
    /// be pulled, so the disposal names none.
    type Disposal = Disposal<S::Mode, T, S::Error, Infallible, S::SubscriptionSlots>;
}

impl<T, S> ObservableTypes for ConcatEager<S, NonZeroUsize>
where
    S: IndexSources<SameItem = T>,
{
    type Item = T;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources still subscribed are disposed the last subscribed first.
    type Disposal = Disposal<S::Mode, T, S::Error, IndexedCursorOf<S>, S::SubscriptionSlots>;
}

/// The cursor over indexed sources of `S`, stored while a limited [`ConcatEager`] is at capacity.
pub type IndexedCursorOf<S> = <<S as IndexSources>::Indexed as SourceSet>::Cursor;

/// The [`Driver`](super::internal::merge::Driver) of the merge a [`ConcatEager`] over `S`,
/// subscribed with `OR`, is made of: its sources made [`IndexedSource`]s, which send their values
/// with their positions, put back in order by [`ReorderObserver`].
type EagerDriverOf<S, OR, C> = DriverOf<
    <S as IndexSources>::Indexed,
    IndexedEvent<<S as SameItemSources>::SameItem>,
    ReorderObserver<<S as SourceSet>::Mode, <S as SameItemSources>::SameItem, OR>,
    C,
>;

impl<S, OR> Observable<OR> for ConcatEager<S>
where
    S: IndexSourcesOps,
    IndexedCursorOf<S>: for<'a> SubscribeEach<&'a EagerDriverOf<S, OR, Infallible>, Pause>,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_unlimited(self.sources.into_indexed_sources(), |stop_view| {
            ReorderObserver::new(observer, stop_view)
        })
        .map_inner_into()
    }
}

impl<S, OR> Observable<OR> for ConcatEager<S, NonZeroUsize>
where
    S: IndexSourcesOps,
    IndexedCursorOf<S>: for<'a> SubscribeEach<&'a EagerDriverOf<S, OR, IndexedCursorOf<S>>, Pause>,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_limited(
            self.sources.into_indexed_sources(),
            self.limit,
            |stop_view| ReorderObserver::new(observer, stop_view),
        )
        .map_inner_into()
    }
}
