use super::from_iter::FromIter;
use crate::utils::types::MaybeSend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;
use std::{convert::Infallible, ops::RangeBounds};

/// Creates an Observable that emits a sequence of integers within a specified range.
/// See <https://reactivex.io/documentation/operators/range.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::creating::range::Range,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// Range::new(1..=3).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Range<I>(I);

impl<I> Range<I> {
    pub fn new<T>(range: I) -> Self
    where
        I: IntoIterator<Item = T> + RangeBounds<T>,
    {
        Self(range)
    }
}

impl<'or, 'sub, T, I> Observable<'or, 'sub, T, Infallible> for Range<I>
where
    I: IntoIterator<Item = T>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + MaybeSend + 'or,
    ) -> Subscription<'sub> {
        FromIter::new(self.0).subscribe(observer)
    }
}
