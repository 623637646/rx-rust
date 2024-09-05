use super::Observable;
use crate::{
    cancellable::Cancellable,
    observer::{anonymous_observer::AnonymousObserver, Event},
};

pub trait ObservableExt<'a, T, E> {
    /// Subscribes to the observable with the given `on_event` callback.
    fn subscribe_on_event<F>(&'a self, on_event: F) -> impl Cancellable
    where
        F: Fn(Event<T, E>) + 'static;

    /// Subscribes to the observable with the given `on_next` callback.
    fn subscribe_on_next<F1>(&'a self, on_next: F1) -> impl Cancellable
    where
        F1: Fn(T) + 'static;
}

impl<'a, T, E, O> ObservableExt<'a, T, E> for O
where
    O: Observable<'a, T, E>,
{
    fn subscribe_on_event<F>(&'a self, on_event: F) -> impl Cancellable
    where
        F: Fn(Event<T, E>) + 'static,
    {
        let observer = AnonymousObserver::new(move |event: Event<T, E>| on_event(event));
        self.subscribe(observer)
    }

    fn subscribe_on_next<F>(&'a self, on_next: F) -> impl Cancellable
    where
        F: Fn(T) + 'static,
    {
        self.subscribe_on_event(move |event: Event<T, E>| {
            if let Event::Next(value) = event {
                on_next(value);
            }
        })
    }
}
