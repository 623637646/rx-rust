//! The [`Just`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    thread_mode::Local,
    utils::emit_then_terminate::emit_then_terminate,
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an observable that emits a single item and then terminates normally.
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
///
/// Just::new("hello").subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec!["hello"]);
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
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        emit_then_terminate(observer, Some(self.0), Termination::Completed);
        DisposeOnDrop::default()
    }
}
