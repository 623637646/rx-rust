//! Emits the items of several sources as they arrive.
//!
//! The [`Merge`] operator, behind
//! [`ObservableExt::merge`](crate::observable::ObservableExt::merge).

use super::internal::merge::{
    ContextDisposal, DriverOf, Pause, subscribe_limited, subscribe_unlimited,
};
use crate::{
    delegate_disposal,
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{
        IterSources, Observable, ObservableTypes, SameItemSources, SourceSet,
        sources::{InternalSourceSet, SubscribeEach},
    },
    observer::Observer,
    thread_mode::ThreadMode,
};
use educe::Educe;
use std::{convert::Infallible, num::NonZeroUsize};

#[doc(hidden)]
pub use super::internal::merge::MergeObserver;

/// Combines multiple observables into a single observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html>
///
/// The sources are a tuple of one to eight observables of one item type and one error type, which
/// may be observables of different types: no boxing is needed to merge them.
/// [`ObservableExt::merge`](crate::observable::ObservableExt::merge) is the form for two;
/// [`Merge::new_from_iter`] is the one for any number of observables of one type, decided as the
/// program runs: any iterator of them, with no tuple arity limit. The stream completes once every
/// source has completed; an error of any of them ends it. The tuple and iterator forms share one
/// state machine.
///
/// [`Merge::max_concurrency`] makes either form subscribe at most `max_concurrency` sources at a
/// time, in order: the next one is pulled — from the tuple, or from the iterator, which is user
/// code — only once one of them has completed, so nothing waits in a queue, and an iterator that
/// never ends still works. The limit is the type parameter `L`: `()`, none, until
/// `max_concurrency` makes it a `NonZeroUsize`. The usual use is to bound work: at most so many requests in flight,
/// however many are asked for. With a `max_concurrency` of one it concatenates the sources, one
/// after the other, which [`Concat`](super::concat::Concat) does for a tuple without a lock. An
/// observable of observables, which pushes them rather than being pulled, goes to
/// [`MergeAll::max_concurrency`](super::merge_all::MergeAll::max_concurrency).
///
/// The limit is in the type: [`Merge`] has none and subscribes every source as it is subscribed
/// itself, while `Merge<S, NonZeroUsize>` holds the sources it has not pulled yet, so its
/// observers and its subscription name their types — they are `Send` in the `Shared` mode only
/// when those sources are, the iterator included, and a value the iterator borrows outlives the
/// subscription. Both release the subscription of a source as it completes, so that a long list
/// holds only those of the sources still running — until an error is queued: from then on, the
/// subscriptions left are released together, as the merge stops.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge::Merge,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Merge::new((
///     FromIter::new([1, 3]),
///     FromIter::new([2, 4]),
///     FromIter::new([5]).map(|value| value * 10),
/// ));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4, 50]);
/// ```
///
/// With a limit, a source is pulled once there is room for it, here from an iterator that never
/// ends:
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::merge::Merge,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, convert::Infallible, num::NonZeroUsize, rc::Rc};
///
/// let mut values = Vec::new();
/// let requests = Rc::new(RefCell::new(Vec::new()));
/// let pulled = requests.clone();
///
/// let observable = Merge::new_from_iter((0..).map(move |_| {
///     let request = PublishSubject::<_, Infallible, _>::local();
///     pulled.borrow_mut().push(request.clone());
///     request
/// }))
/// .max_concurrency(NonZeroUsize::new(2).unwrap());
/// let subscription = observable.subscribe_with_callback(|value| values.push(value), |_| {});
/// assert_eq!(requests.borrow().len(), 2); // Pulled as there is room for them.
///
/// let mut first = requests.borrow()[0].clone();
/// let _ = first.on_next(1);
/// first.on_termination(Termination::Completed); // Makes room for the third one.
/// assert_eq!(requests.borrow().len(), 3);
///
/// drop((subscription, requests));
/// assert_eq!(values, [1]);
/// ```
///
/// The mode is the sources' joined ([`SourceSet::Mode`]): a merge of
/// `Local` sources keeps its state in an `Rc` and takes an observer holding one,
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .merge(PublishSubject::local())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
///
/// while a `Shared` source makes the whole merge `Shared`, and its observer must be `Send`:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .merge(PublishSubject::shared())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Merge<S, L = ()> {
    sources: S,
    limit: L,
}

impl<S> Merge<S> {
    /// Creates a [`Merge`] over the tuple `sources`;
    /// [`ObservableExt::merge`](crate::observable::ObservableExt::merge) is the fluent
    /// form for two.
    pub fn new(sources: S) -> Self
    where
        S: SameItemSources,
    {
        Self { sources, limit: () }
    }

    /// Subscribes at most `max_concurrency` of the sources at a time: the next one is pulled,
    /// from the tuple or from the iterator, only once one of them has completed, so an iterator
    /// that never ends can be merged. With a `max_concurrency` of one the sources are
    /// concatenated.
    pub fn max_concurrency(self, max_concurrency: NonZeroUsize) -> Merge<S, NonZeroUsize> {
        Merge {
            sources: self.sources,
            limit: max_concurrency,
        }
    }
}

impl<I> Merge<IterSources<I>> {
    /// Merges the observables of an iterator, subscribing as it is consumed.
    ///
    /// Stops pulling sources when the stream stops; an empty iterator completes immediately.
    pub fn new_from_iter<OE>(sources: I) -> Self
    where
        I: IntoIterator<Item = OE>,
        OE: ObservableTypes,
    {
        Self {
            sources: IterSources(sources),
            limit: (),
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, C, SL>,
    ContextDisposal<M, T, E, C, SL>,
    where M: ThreadMode
);

impl<T, S> ObservableTypes for Merge<S>
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

impl<T, S> ObservableTypes for Merge<S, NonZeroUsize>
where
    S: SameItemSources<SameItem = T>,
{
    type Item = T;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources still subscribed are disposed the last subscribed first.
    type Disposal = Disposal<S::Mode, T, S::Error, S::Cursor, S::SubscriptionSlots>;
}

/// The [`Driver`](super::internal::merge::Driver) of a [`Merge`] over `S`, subscribed with `OR`,
/// whose sources wait for room as a `C`.
type MergeDriverOf<S, OR, C> = DriverOf<S, <S as SameItemSources>::SameItem, OR, C>;

impl<S, OR> Observable<OR> for Merge<S>
where
    S: SameItemSources + InternalSourceSet,
    <S as SourceSet>::Cursor: for<'a> SubscribeEach<&'a MergeDriverOf<S, OR, Infallible>, Pause>,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_unlimited(self.sources, |_| observer).map_inner_into()
    }
}

impl<S, OR> Observable<OR> for Merge<S, NonZeroUsize>
where
    S: SameItemSources + InternalSourceSet,
    <S as SourceSet>::Cursor:
        for<'a> SubscribeEach<&'a MergeDriverOf<S, OR, <S as SourceSet>::Cursor>, Pause>,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_limited(self.sources, self.limit, |_| observer).map_inner_into()
    }
}
