use super::from_iter::FromIter;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, ops::RangeBounds};

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
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        FromIter::new(self.0).subscribe(observer)
    }
}

impl<I> ObservableExt for Range<I> {}
