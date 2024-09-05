pub mod anonymous_observer;

#[derive(Debug, PartialEq, Eq)]
pub enum Terminated<E> {
    Error(E),
    Disposed,
    Completed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event<T, E> {
    Next(T),
    Terminated(Terminated<E>),
}

pub trait Observer<T, E> {
    // TODO: use mutable reference?
    fn on(&self, event: Event<T, E>);
}
