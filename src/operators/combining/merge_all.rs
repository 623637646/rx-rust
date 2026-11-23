//! The [`MergeAll`] operator, behind
//! [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all).

use super::internal::flatten::{self, ContextDisposal, InnerObserverOf, SourceObserverOf};
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

pub use super::internal::flatten::{Enqueue, Identity, InnerObserver, SourceObserver};

/// Flattens an observable of observables by emitting the values of the inner observables as they
/// arrive.
/// See <https://reactivex.io/documentation/operators/merge.html>
///
/// [`MergeAll::new`] subscribes every inner observable as it arrives.
/// [`MergeAll::max_concurrency`] subscribes at most `max_concurrency` of them at a time: one
/// that arrives while that many are running waits in a queue, and is subscribed once one of them
/// completes, in the order they arrived. With a `max_concurrency` of one it is
/// [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll). The usual use is to bound
/// work: at most so many requests in flight, however many are asked for.
/// That limit bounds active subscriptions only: the source pushes its observables when it likes,
/// so the waiting queue can grow without a bound.
///
/// The observables of a tuple or a list go to [`Merge`](super::merge::Merge), with a limit too,
/// which pulls them only as there is room for them and so queues nothing.
///
/// The stream completes once the source and every inner observable, the queued ones included, have
/// completed; an error of any of them ends it, and drops the queue.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge_all::MergeAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = MergeAll::new(FromIter::new([
///     FromIter::new([1, 3]),
///     FromIter::new([2, 4]),
/// ]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// ```
///
/// With a limit, an inner observable waits for a place before it is subscribed:
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, num::NonZeroUsize};
///
/// let mut values = Vec::new();
/// let mut requests = PublishSubject::local();
/// let mut request_1 = PublishSubject::<_, Infallible, _>::local();
/// let mut request_2 = PublishSubject::local();
/// let mut request_3 = PublishSubject::local();
///
/// let subscription = requests
///     .clone()
///     .merge_all()
///     .max_concurrency(NonZeroUsize::new(2).unwrap())
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
/// for request in [&request_1, &request_2, &request_3] {
///     let _ = requests.on_next(request.clone());
/// }
///
/// let _ = request_3.on_next(3); // Queued, not subscribed yet: missed.
/// let _ = request_2.on_next(2);
/// request_1.on_termination(Termination::Completed); // Makes room for the third one.
/// let _ = request_3.on_next(33);
///
/// drop((subscription, requests, request_2, request_3));
/// assert_eq!(values, [2, 33]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MergeAll<OE, OE1> {
    source: OE,
    max_concurrency: NonZeroUsize,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> MergeAll<OE, OE1> {
    /// Creates a [`MergeAll`] over `source`, with no limit;
    /// [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all) is the fluent
    /// form.
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
    ContextDisposal<M, T, E, D, Enqueue<OE1>, <OE1 as ObservableTypes>::Disposal>,
    where M: ThreadMode, D: Disposable, OE1: ObservableTypes
);

impl<T, E, OE, OE1> ObservableTypes for MergeAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for MergeAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SourceObserverOf<OE, OE1, T, E, OR, Enqueue<OE1>, Identity>,
            Item = OE1,
            Error = E,
        >,
    OE1: Observable<InnerObserverOf<OE, OE1, T, E, OR, Enqueue<OE1>>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        flatten::subscribe(
            self.source,
            |_| observer,
            self.max_concurrency,
            Enqueue::new(),
            Identity,
        )
        .map_inner_into()
    }
}
