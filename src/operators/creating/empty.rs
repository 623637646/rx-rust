use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Empty;

impl<'or, 'sub> Observable<'or, 'sub, Infallible, Infallible> for Empty {
    fn subscribe(
        self,
        observer: impl Observer<Infallible, Infallible> + Send + 'or,
    ) -> Subscription<'sub> {
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    }
}
