use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnSubscription<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnSubscription<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(OE, BoxedObserver<'or, T, E>) -> Subscription<'sub>,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnSubscription<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(OE, BoxedObserver<'or, T, E>) -> Subscription<'sub>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        (self.callback)(self.source, BoxedObserver::new(observer))
    }
}
