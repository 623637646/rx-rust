//! The [`FromResult`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    thread_mode::Local,
};
use educe::Educe;

/// Converts a `Result` into an observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::from_result::FromResult,
/// };
///
/// let mut values = Vec::new();
///
/// FromResult::new(Ok::<i32, &str>(10)).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![10]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromResult<T, E>(Result<T, E>);

impl<T, E> FromResult<T, E> {
    /// Creates a [`FromResult`].
    pub fn new(result: Result<T, E>) -> Self {
        Self(result)
    }
}

impl<T, E> ObservableTypes for FromResult<T, E> {
    type Item = T;
    type Error = E;
    type Mode = Local;
    type Disposal = ();
}

impl<T, E, OR> Observable<OR> for FromResult<T, E>
where
    OR: Observer<T, E>,
{
    fn subscribe(self, mut observer: OR) -> DisposeOnDrop<Self::Disposal> {
        match self.0 {
            Ok(value) => {
                if observer.on_next(value).is_continue() {
                    observer.on_termination(Termination::Completed);
                }
            }
            Err(error) => observer.on_termination(Termination::Error(error)),
        }
        DisposeOnDrop::default()
    }
}
