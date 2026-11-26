//! Gathers items into `Vec`s of a given length that overlap, or leave gaps.
//!
//! The [`BufferWithCountAndSkip`] operator, behind
//! [`ObservableExt::buffer_with_count_and_skip`](crate::observable::ObservableExt::buffer_with_count_and_skip).

use super::internal::{
    bundle::Bundles,
    count::CountObserver,
    overlap::{ByClone, Duplicate},
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;
use std::num::NonZeroUsize;

/// Gathers the items of an observable into bundles of `count` items, starting a new bundle every
/// `skip` items, and emits each as a `Vec<T>` once it is full; the bundles still open when the
/// source completes are emitted then, smaller.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// With `skip` smaller than `count` the bundles overlap — a sliding window over the stream, each
/// item in up to `count.div_ceil(skip)` bundles, cloned into all of them but the newest — and with
/// `skip` larger than `count` the items between two bundles are dropped. With `skip` equal to
/// `count` it is
/// [`BufferWithCount`](crate::operators::transforming::buffer_with_count::BufferWithCount), which
/// does not need the items to be [`Clone`].
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::buffer_with_count_and_skip::BufferWithCountAndSkip,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
///
/// let observable = BufferWithCountAndSkip::new(
///     FromIter::new([1, 2, 3, 4]),
///     NonZeroUsize::new(3).unwrap(),
///     NonZeroUsize::new(1).unwrap(),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4], vec![4]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCountAndSkip<OE> {
    source: OE,
    count: NonZeroUsize,
    skip: NonZeroUsize,
}

impl<OE> BufferWithCountAndSkip<OE> {
    /// Creates a [`BufferWithCountAndSkip`] over `source`;
    /// [`ObservableExt::buffer_with_count_and_skip`](crate::observable::ObservableExt::buffer_with_count_and_skip)
    /// is the fluent form.
    pub fn new(source: OE, count: NonZeroUsize, skip: NonZeroUsize) -> Self {
        Self {
            source,
            count,
            skip,
        }
    }
}

impl<T, E, OE> ObservableTypes for BufferWithCountAndSkip<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

/// The observer a [`BufferWithCountAndSkip`] gives its source.
pub(crate) type BufferWithCountAndSkipObserver<T, OR> = CountObserver<Bundles<T, ByClone>, OR>;

impl<T, E, OE, OR> Observable<OR> for BufferWithCountAndSkip<OE>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<BufferWithCountAndSkipObserver<T, OR>, Item = T, Error = E>,
    T: Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe(self.source, observer, self.count, self.skip, ByClone)
    }
}

/// Subscribes `source` with the observer of a [`BufferWithCountAndSkip`], or, with a `skip` equal
/// to `count` and [`NoOverlap`](super::internal::overlap::NoOverlap) as the duplication, of a
/// [`BufferWithCount`](super::buffer_with_count::BufferWithCount).
pub(super) fn subscribe<T, E, OE, OR, DU>(
    source: OE,
    observer: OR,
    count: NonZeroUsize,
    skip: NonZeroUsize,
    duplicate: DU,
) -> DisposeOnDrop<OE::Disposal>
where
    OR: Observer<Vec<T>, E>,
    DU: Duplicate<T>,
    OE: Observable<CountObserver<Bundles<T, DU>, OR>, Item = T, Error = E>,
{
    let mut observer = CountObserver::new(
        Bundles::with_room(duplicate, count.get()),
        observer,
        count,
        skip,
    );
    let flow = observer.open_first::<E>();
    debug_assert!(flow.is_continue(), "a bundle emits nothing as it opens");
    source.subscribe(observer)
}
