use super::from_iter::FromIter;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Repeat<T> {
    value: T,
    n: usize,
}

impl<T> Repeat<T> {
    pub fn new(value: T, n: usize) -> Self {
        Self { value, n }
    }
}

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Repeat<T>
where
    T: Clone,
{
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        FromIter::new(std::iter::repeat_n(self.value, self.n)).subscribe(observer)
    }
}

