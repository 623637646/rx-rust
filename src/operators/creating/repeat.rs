//! The [`Repeat`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    operators::creating::from_iter::FromIter,
    thread_mode::Local,
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits a particular item multiple times.
/// See <https://reactivex.io/documentation/operators/repeat.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::repeat::Repeat,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// Repeat::new("ping", 3).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec!["ping", "ping", "ping"]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Repeat<T> {
    value: T,
    count: usize,
}

impl<T> Repeat<T> {
    /// Creates a [`Repeat`].
    pub fn new(value: T, count: usize) -> Self {
        Self { value, count }
    }
}

impl<T> ObservableTypes for Repeat<T>
where
    T: Clone,
{
    type Item = T;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<T, OR> Observable<OR> for Repeat<T>
where
    OR: Observer<T, Infallible>,
    T: Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        FromIter::new(std::iter::repeat_n(self.value, self.count)).subscribe(observer)
    }
}
