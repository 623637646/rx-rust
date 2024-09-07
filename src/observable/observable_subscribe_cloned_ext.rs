use super::observable_cloned::ObservableCloned;
use crate::{
    cancellable::Cancellable,
    observer::{anonymous_observer::AnonymousObserver, Event},
};

pub trait ObservableSubscribeClonedExt<T, E> {
    /// Subscribes to the observable with the given `on_event` callback.
    fn subscribe_cloned_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: Fn(Event<T, E>) + 'static;

    /// Subscribes to the observable with the given `on_next` callback.
    fn subscribe_cloned_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: Fn(T) + 'static;
}

impl<T, E, O> ObservableSubscribeClonedExt<T, E> for O
where
    O: ObservableCloned<T, E>,
{
    fn subscribe_cloned_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: Fn(Event<T, E>) + 'static,
    {
        let observer = AnonymousObserver::new(on_event);
        self.subscribe_cloned(observer)
    }

    fn subscribe_cloned_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: Fn(T) + 'static,
    {
        self.subscribe_cloned_on_event(move |event: Event<T, E>| {
            if let Event::Next(value) = event {
                on_next(value);
            }
        })
    }
}
