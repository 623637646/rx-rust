pub mod create;
pub mod error;
pub mod just;
pub mod map;

use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observer::{anonymous_observer::AnonymousObserver, Event, Observer, Terminated},
};
use create::Create;
use map::Map;

pub trait Observable<'a, T, E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<T, E>;

    fn subscribe_on<F1, F2>(&'a self, on_next: F1, on_terminated: F2) -> impl Disposable
    where
        F1: Fn(T) + 'static,
        F2: Fn(Terminated<E>) + 'static,
    {
        // TODO use disposable
        // let dispose: Box<dyn Disposable> = match on_disposed {
        //     Some(f) => Box::new(AnonymousDisposable::new(f)),
        //     None => Box::new(NopDisposable::new()),
        // };
        let dispose = NopDisposable::new();

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

// TODO: here
// trait Cloned<'a, T, E> {
//     fn cloned(&self) -> impl Observable<T, E>;
// }

// impl<'a, T, E, O> Cloned<'a, T, E> for O
// where
//     O: Observable<'a, T, E>,
// {
//     fn cloned(&self) -> impl Observable<T, E> {
//         Create::new(|observer: &dyn Observer<T, E>| || {})
//     }
// }

// TODO: create Rx, using API like Rx::just, Rx::empty
