pub mod anonymous_observer;

/// A `Terminated` is a value that an `Observable` can send to an `Observer` to indicate that the observable has terminated.
#[derive(Debug, PartialEq, Eq)]
pub enum Terminated<E> {
    Error(E),
    Cancelled,
    Completed,
}

/// An `Event` is a value that an `Observable` can send to an `Observer`.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<T, E> {
    Next(T),
    Terminated(Terminated<E>),
}

impl<T, E> Event<T, E> {
    pub fn map_next<T2, F>(self, f: F) -> Event<T2, E>
    where
        F: Fn(T) -> T2,
    {
        match self {
            Event::Next(value) => Event::Next(f(value)),
            Event::Terminated(terminated) => Event::Terminated(terminated),
        }
    }

    pub fn map_error<E2, F>(self, f: F) -> Event<T, E2>
    where
        F: Fn(E) -> E2,
    {
        match self {
            Event::Next(value) => Event::Next(value),
            Event::Terminated(terminated) => match terminated {
                Terminated::Error(error) => Event::Terminated(Terminated::Error(f(error))),
                Terminated::Cancelled => Event::Terminated(Terminated::Cancelled),
                Terminated::Completed => Event::Terminated(Terminated::Completed),
            },
        }
    }
}

/// An `Observer` is a type that can receive events from an `Observable`.
pub trait Observer<T, E> {
    fn on(&self, event: Event<T, E>);
}
