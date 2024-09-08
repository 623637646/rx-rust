use super::{Event, Observer};

/// An observer that wraps a closure.
pub struct AnonymousObserver<F> {
    on_event: F,
}

impl<F> AnonymousObserver<F> {
    pub fn new(on_event: F) -> AnonymousObserver<F> {
        AnonymousObserver { on_event }
    }
}

impl<T, E, F> Observer<T, E> for AnonymousObserver<F>
where
    F: FnMut(Event<T, E>),
{
    fn on(&mut self, event: Event<T, E>) {
        (self.on_event)(event);
    }
}
