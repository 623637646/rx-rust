use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Converts a `Result` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::creating::from_result::FromResult,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// FromResult::new(Ok::<i32, &str>(10)).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![10]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromResult<T, E>(Result<T, E>);

impl<T, E> FromResult<T, E> {
    pub fn new(result: Result<T, E>) -> Self {
        Self(result)
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for FromResult<T, E> {
    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        match self.0 {
            Ok(value) => {
                observer.on_next(value);
                observer.on_termination(Termination::Completed);
            }
            Err(error) => observer.on_termination(Termination::Error(error)),
        }
        Subscription::default()
    }
}
