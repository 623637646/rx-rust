use crate::operators::creating::defer::Defer;
use crate::operators::creating::just::Just;
use crate::utils::types::NecessarySend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits the return value of a function.
/// See <https://reactivex.io/documentation/operators/start.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Start<T, F>(F)
where
    F: FnOnce() -> T;

impl<T, F> Start<T, F>
where
    F: FnOnce() -> T,
{
    pub fn new(builder: F) -> Self {
        Self(builder)
    }
}

impl<'or, 'sub, T, F> Observable<'or, 'sub, T, Infallible> for Start<T, F>
where
    F: FnOnce() -> T,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        Defer::new(|| Just::new(self.0())).subscribe(observer)
    }
}
