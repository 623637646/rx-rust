use super::Observable;
use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observer::{anonymous_observer::AnonymousObserver, Event, Terminated},
};

pub trait ObservableExt<'a, T, E> {
    fn subscribe_on<F1, F2>(&'a self, on_next: F1, on_terminated: F2) -> impl Disposable
    where
        F1: Fn(T) + 'static,
        F2: Fn(Terminated<E>) + 'static;

    fn subscribe_on_next<F1>(&'a self, on_next: F1) -> impl Disposable
    where
        F1: Fn(T) + 'static;
}

impl<'a, T, E, O> ObservableExt<'a, T, E> for O
where
    O: Observable<'a, T, E>,
{
    fn subscribe_on<F1, F2>(&'a self, on_next: F1, on_terminated: F2) -> impl Disposable
    where
        F1: Fn(T) + 'static,
        F2: Fn(Terminated<E>) + 'static,
    {
        // TODO use disposable
        // let dispose: Box<dyn Disposable> = match on_disposed {
        //     Some(f) => Box::new(AnonymousDisposable::new(f)),
        //     None => Box::new(NopDisposable{}),
        // };
        let dispose = NopDisposable {};

        let observer = AnonymousObserver::new(move |event: Event<T, E>| match event {
            Event::Next(value) => on_next(value),
            Event::Terminated(terminated) => on_terminated(terminated),
        });

        self.subscribe(observer);
        dispose
    }

    fn subscribe_on_next<F1>(&'a self, on_next: F1) -> impl Disposable
    where
        F1: Fn(T) + 'static,
    {
        self.subscribe_on(on_next, |_| {})
    }
}
