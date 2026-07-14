use crate::operators::creating::defer::Defer;
use crate::operators::creating::just::Just;
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
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
    pub fn new<T>(builder: F) -> Self
    where
        F: FnOnce() -> T,
    {
        Self(builder)
    }
}

impl<'or, T, F> Observable<'or> for Start<F>
where
    F: FnOnce() -> T,
{
    type T = T;
    type E = Infallible;
    type D = ();

    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        Defer::new(|| Just::new(self.0())).subscribe(observer)
    }
}
