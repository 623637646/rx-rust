use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Empty;

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Empty {
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    }
}

impl ObservableExt for Empty {}
