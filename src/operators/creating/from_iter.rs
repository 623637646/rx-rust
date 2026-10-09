//! The [`FromIter`] source.

use crate::thread_mode::Local;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an `IntoIterator` into an Observable.
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
/// let mut terminations = Vec::new();
///
/// FromIter::new(vec![1, 2, 3]).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
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
    fn subscribe(self, mut observer: OR) -> DisposeOnDrop<Self::Disposal> {
        for value in self.0.into_iter() {
            if observer.on_next(value).is_stop() {
                // The observer stopped: stop iterating, which is the only way to end an infinite
                // iterator (the subscription only exists once this returns), and release the
                // observer without a termination, like a disposed one.
                return DisposeOnDrop::default();
            }
        }
        observer.on_termination(Termination::Completed);
        DisposeOnDrop::default()
    }
}
