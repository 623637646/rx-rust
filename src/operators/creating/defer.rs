use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Defer<OE, F>(F)
where
    F: FnOnce() -> OE;

impl<OE, F> Defer<OE, F>
where
    F: FnOnce() -> OE,
{
    pub fn new(builder: F) -> Self {
        Self(builder)
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for Defer<OE, F>
where
    F: FnOnce() -> OE,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observable = self.0();
        observable.subscribe(observer)
    }
}
