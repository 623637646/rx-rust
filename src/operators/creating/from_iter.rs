use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an `IntoIterator` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::from_iter::FromIter,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// FromIter::new(vec![1, 2, 3]).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromIter<I>(I);

impl<I> FromIter<I> {
    pub fn new(into_iterator: I) -> Self {
        Self(into_iterator)
    }
}

impl<'or, T, I> Observable<'or> for FromIter<I>
where
    I: IntoIterator<Item = T>,
{
    type T = T;
    type E = Infallible;
    type D = ();

    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        for value in self.0.into_iter() {
            observer.on_next(value);
        }
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
