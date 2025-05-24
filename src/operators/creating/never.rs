use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Never {
    fn subscribe(self, _: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        Subscription::new_none_disposal()
    }
}

