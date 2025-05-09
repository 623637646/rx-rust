use super::just::Just;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Start<T>(Just<T>);

impl<T> Start<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T,
    {
        Self(Just::new(f()))
    }
}

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Start<T> {
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        self.0.subscribe(observer)
    }
}

impl<T> ObservableExt for Start<T> {}
