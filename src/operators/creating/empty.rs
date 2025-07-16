use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Empty;

impl<'or, 'sub> Observable<'or, 'sub, Infallible, Infallible> for Empty {
    fn subscribe(
        self,
        observer: impl Observer<Infallible, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
