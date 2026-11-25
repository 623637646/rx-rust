//! Emits the values of a range.
//!
//! The [`Range`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    operators::creating::from_iter::FromIter,
    thread_mode::Local,
};
use educe::Educe;
use std::{convert::Infallible, ops::RangeBounds};

/// Creates an observable that emits the values of an iterable range.
/// See <https://reactivex.io/documentation/operators/range.html>
///
/// The input must implement both [`IntoIterator`] and [`RangeBounds`], as `1..4` and `1..=3` do. A
/// standard open-ended range such as `1..` has no completion event. Use [`FromIter`] for iterables
/// without range bounds.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::range::Range,
/// };
///
/// let mut values = Vec::new();
///
/// let _subscription = Range::new(1..=3).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// ```
///
/// An array can be iterated, but it is not a range:
/// ```rust
/// use rx_rust::{observable::ObservableExt, operators::creating::range::Range};
///
/// let _subscription = Range::new(1..=3).subscribe_with_callback(|_: i32| {}, |_| {});
/// ```
/// ```rust,compile_fail
/// use rx_rust::{observable::ObservableExt, operators::creating::range::Range};
///
/// let _subscription = Range::new([1, 2, 3]).subscribe_with_callback(|_: i32| {}, |_| {});
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Range<I>(I);

impl<I> Range<I> {
    /// Creates a [`Range`].
    pub fn new(range: I) -> Self {
        Self(range)
    }
}

impl<T, I> ObservableTypes for Range<I>
where
    I: IntoIterator<Item = T>,
{
    type Item = T;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<T, I, OR> Observable<OR> for Range<I>
where
    OR: Observer<T, Infallible>,
    I: IntoIterator<Item = T> + RangeBounds<T>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        FromIter::new(self.0).subscribe(observer)
    }
}
