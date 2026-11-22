//! The [`FromIter`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    thread_mode::Local,
    utils::emit_then_terminate::emit_then_terminate,
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an `IntoIterator` into an observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::from_iter::FromIter,
/// };
///
/// let mut values = Vec::new();
///
/// FromIter::new([1, 2, 3]).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromIter<I>(I);

impl<I> FromIter<I> {
    /// Creates a [`FromIter`].
    pub fn new(into_iterator: I) -> Self {
        Self(into_iterator)
    }
}

impl<T, I> ObservableTypes for FromIter<I>
where
    I: IntoIterator<Item = T>,
{
    type Item = T;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<T, I, OR> Observable<OR> for FromIter<I>
where
    OR: Observer<T, Infallible>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // The subscription only exists once this returns, so an infinite iterator can only
        // stop when the observer breaks.
        emit_then_terminate(observer, self.0, Termination::Completed);
        DisposeOnDrop::default()
    }
}
