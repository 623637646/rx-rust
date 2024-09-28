#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Terminal<E> {
    Completed,
    Error(E),
}

/// An `Observer` is a type that can receive events from an `Observable`.
pub trait Observer<T, E> {
    fn on_next(&mut self, value: T);

    fn on_terminal(self, terminal: Terminal<E>);
}
