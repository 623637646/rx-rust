use super::Observable;
use crate::{
    cancellable::Cancellable,
    observer::{anonymous_observer::AnonymousObserver, Event},
};

pub trait ObservableSubscribeExt<T, E> {
    /// Subscribes to the observable with the given `on_event` callback.
    fn subscribe_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(Event<&'a T, E>) + 'static;

    /// Subscribes to the observable with the given `on_next` callback.
    fn subscribe_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(&'a T) + 'static;
}

impl<T, E, O> ObservableSubscribeExt<T, E> for O
where
    O: Observable<T, E>,
{
    fn subscribe_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(Event<&'a T, E>) + 'static,
    {
        let observer = AnonymousObserver::new(on_event);
        self.subscribe(observer)
    }

    fn subscribe_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(&'a T) + 'static,
    {
        self.subscribe_on_event(move |event| {
            if let Event::Next(value) = event {
                on_next(value);
            }
        })
    }
}
