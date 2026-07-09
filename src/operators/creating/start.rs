use crate::operators::creating::defer::Defer;
use crate::operators::creating::just::Just;
use crate::utils::types::MaybeSend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits the return value of a function.
/// See <https://reactivex.io/documentation/operators/start.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
pub struct Start<T, F>(F)
where
    F: FnOnce() -> T;

impl<T, F> Start<T, F>
where
    F: FnOnce() -> T,
{
    pub fn new(builder: F) -> Self {
        Self(builder)
    }
}

impl<'or, 'sub, T, F> Observable<'or, 'sub, T, Infallible> for Start<T, F>
where
    F: FnOnce() -> T,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<'sub> {
        Defer::new(|| Just::new(self.0())).subscribe(observer)
    }
}
