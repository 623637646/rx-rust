use crate::utils::types::NecessarySend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits no items and never terminates.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'or, 'sub> Observable<'or, 'sub, Infallible, Infallible> for Never {
    fn subscribe(
        self,
        _: impl Observer<Infallible, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        Subscription::default()
    }
}
