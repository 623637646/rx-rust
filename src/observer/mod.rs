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

/// An `Observer` is a type that can receive events from an `Observable`.
pub trait Observer<T, E> {
    // TODO: use mutable reference?
    fn on(&self, event: Event<T, E>);
}
