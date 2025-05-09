use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnTermination<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoOnTermination<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(&Termination<E>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = DoOnTerminationObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

impl<OE, F> ObservableExt for DoOnTermination<OE, F> {}

struct DoOnTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoOnTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(&Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(&termination);
        self.observer.on_termination(termination);
    }
}
