//! The [`Start`] source.

use crate::operators::creating::defer::Defer;
use crate::operators::creating::just::Just;
use crate::thread_mode::Local;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits the return value of a function.
/// See <https://reactivex.io/documentation/operators/start.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::start::Start,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// Start::new(|| 21 + 21).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Start<F>(F);

impl<F> Start<F> {
    /// Creates a [`Start`].
    pub fn new<T>(builder: F) -> Self
    where
        F: FnOnce() -> T,
    {
        Self(builder)
    }
}

impl<T, F> ObservableTypes for Start<F>
where
    F: FnOnce() -> T,
{
    type Item = T;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<T, F, OR> Observable<OR> for Start<F>
where
    OR: Observer<T, Infallible>,
    F: FnOnce() -> T,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        Defer::new(|| Just::new(self.0())).subscribe(observer)
    }
}
