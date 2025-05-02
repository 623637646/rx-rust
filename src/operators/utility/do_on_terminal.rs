use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnTerminal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnTerminal<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Terminal<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoOnTerminal<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(&Terminal<E>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = DoOnTerminalObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct DoOnTerminalObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoOnTerminalObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(&Terminal<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.callback)(&terminal);
        self.observer.on_terminal(terminal);
    }
}
