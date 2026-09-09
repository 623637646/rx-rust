use crate::operators::creating::from_iter::FromIter;
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
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
    n: usize,
}

impl<T> Repeat<T> {
    pub fn new(value: T, n: usize) -> Self
    where
        T: Clone,
    {
        Self { value, n }
    }
}

impl<'or, T> Observable<'or, T, Infallible> for Repeat<T>
where
    T: Clone,
{
    type D = ();

    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        FromIter::new(std::iter::repeat_n(self.value, self.n)).subscribe(observer)
    }
}
