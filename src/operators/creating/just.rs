use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
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
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<'or, T> Observable<'or, T, Infallible> for Just<T> {
    type D = ();

    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        if observer.on_next(self.0).is_continue() {
            observer.on_termination(Termination::Completed);
        }
        Subscription::default()
    }
}
