//! The [`Just`] source.

use crate::thread_mode::Local;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits a single item and then terminates normally.
/// See <https://reactivex.io/documentation/operators/just.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::just::Just,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// Just::new("hello").subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec!["hello"]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Just<T>(T);

impl<T> Just<T> {
    /// Creates a [`Just`].
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> ObservableTypes for Just<T> {
    type Item = T;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<T, OR> Observable<OR> for Just<T>
where
    OR: Observer<T, Infallible>,
{
    fn subscribe(self, mut observer: OR) -> DisposeOnDrop<Self::Disposal> {
        if observer.on_next(self.0).is_continue() {
            observer.on_termination(Termination::Completed);
        }
        DisposeOnDrop::default()
    }
}
