//! The [`BufferWithCount`] operator, behind
//! [`ObservableExt::buffer_with_count`](crate::observable::ObservableExt::buffer_with_count).

use super::{buffer_with_count_and_skip, internal::overlap::no_overlap};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;
use std::num::NonZeroUsize;

#[doc(hidden)]
pub use super::buffer_with_count_and_skip::BufferWithCountAndSkipObserver as BufferWithCountObserver;

/// Gathers the items of an observable into bundles of a specified size, emitted as `Vec<T>`; the
/// last bundle, emitted on completion, may be smaller.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::buffer_with_count::BufferWithCount,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
///
/// let observable =
///     BufferWithCount::new(FromIter::new([1, 2, 3, 4]), NonZeroUsize::new(3).unwrap());
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![vec![1, 2, 3], vec![4]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> BufferWithCount<OE> {
    /// Creates a [`BufferWithCount`] over `source`;
    /// [`ObservableExt::buffer_with_count`](crate::observable::ObservableExt::buffer_with_count) is
    /// the fluent form.
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<T, E, OE> ObservableTypes for BufferWithCount<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for BufferWithCount<OE>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<BufferWithCountObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // A `BufferWithCountAndSkip` whose bundles start as the one before is full: a single one
        // is ever open, so no item is cloned (decision 0014).
        buffer_with_count_and_skip::subscribe(
            self.source,
            observer,
            self.count,
            self.count,
            no_overlap,
        )
    }
}
