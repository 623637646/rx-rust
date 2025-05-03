use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnNext<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoOnNext<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = DoOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct DoOnNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoOnNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(&value);
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.observer.on_terminal(terminal);
    }
}
