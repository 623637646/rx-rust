use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source Observable.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnNext<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&mut dyn Observer<T, E>, T),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnNext<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&mut dyn Observer<T, E>, T) + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = HookOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct HookOnNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&mut dyn Observer<T, E>, T),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(&mut self.observer, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
