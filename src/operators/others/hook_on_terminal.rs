use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnTerminal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnTerminal<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnTerminal<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = HookOnTerminalObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct HookOnTerminalObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnTerminalObserver<OR, F>
where
    OR: Observer<T, E>,
    F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.callback)(
            terminal,
            Box::new(|terminal| {
                self.observer.on_terminal(terminal);
            }),
        );
    }
}
