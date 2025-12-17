use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
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
///     observable::observable_ext::ObservableExt,
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

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Just<T> {
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        observer.on_next(self.0);
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
