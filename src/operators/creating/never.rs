use crate::utils::types::NecessarySend;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'or, 'sub> Observable<'or, 'sub, Infallible, Infallible> for Never {
    fn subscribe(
        self,
        _: impl Observer<Infallible, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        Subscription::new_none_disposal()
    }
}
