use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;

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
        F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnNext<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = HookOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

impl<OE, F> ObservableExt for HookOnNext<OE, F> {}

struct HookOnNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(
            value,
            Box::new(|value| {
                self.observer.on_next(value);
            }),
        );
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
