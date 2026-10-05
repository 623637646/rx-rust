//! The [`Range`] source.

use crate::operators::creating::from_iter::FromIter;
use crate::thread_mode::Local;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits a sequence of integers within a specified range.
/// See <https://reactivex.io/documentation/operators/range.html>
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
/// let mut terminations = Vec::new();
///
/// Range::new(1..=3).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
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
    type D = ();
}

impl<T, I, OR> Observable<OR> for Range<I>
where
    OR: Observer<T, Infallible>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        FromIter::new(self.0).subscribe(observer)
    }
}
