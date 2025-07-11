use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnTermination<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(BoxedObserver<'or, T, E>, Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnTermination<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(BoxedObserver<'or, T, E>, Termination<E>) + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = HookOnTerminationObserver {
            observer: BoxedObserver::new(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct HookOnTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(OR, Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(self.observer, termination);
    }
}
