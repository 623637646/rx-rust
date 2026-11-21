//! The [`ConcatAll`] operator, behind
//! [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all).

pub use super::internal::flatten::{Enqueue, Identity, InnerObserver, SourceObserver};
use super::{
    internal::flatten::{self, InnerObserverOf, SourceObserverOf},
    merge_all,
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Flattens an observable of observables by emitting all of the values of each inner observable,
/// one inner observable after the other.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// It is [`MergeAll`](super::merge_all::MergeAll) with a `max_concurrency` of one.
/// An observable of observables pushes them, so the ones that arrive while another is subscribed
/// wait in a queue, which has no bound. The
/// observables of a tuple or of a list are pulled instead, one once the previous one has
/// completed: they go to [`Concat`](super::concat::Concat).
///
/// The stream completes once the source and every inner observable, the queued ones included, have
/// completed; an error of the source or of the running inner observable ends it, and drops the
/// queue.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat_all::ConcatAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = ConcatAll::new(FromIter::new([
///     FromIter::new([1, 2]),
///     FromIter::new([3, 4]),
/// ]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatAll<OE, OE1> {
    /// Creates a [`ConcatAll`] over `source`;
    /// [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all) is the fluent
    /// form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE, OE1> ObservableTypes for ConcatAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// The disposal of the [`MergeAll`](super::merge_all::MergeAll) this operator is one of.
    type Disposal = merge_all::Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for ConcatAll<OE, OE1>
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
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Enqueue::new(),
            Identity,
        )
        .map_inner_into()
    }
}
