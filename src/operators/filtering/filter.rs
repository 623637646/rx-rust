use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Filter<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Filter<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for Filter<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> bool + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = FilterObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct FilterObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for FilterObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) {
        if (self.callback)(&value) {
            self.observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
