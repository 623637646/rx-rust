pub mod anonymous_observer;
pub mod observer_ext;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Terminal<E> {
    Error(E),
    Completed,
}

/// An `Observer` is a type that can receive events from an `Observable`.
/// The observer must be Sync and Send because it will be used in multiple threads. See Scheduler usage in delay.rs.
/// The observer must be 'static because it will be stored in Subscription or hot observables.
pub trait Observer<T, E>: Sync + Send + 'static {
    /// Received a value.
    fn on_next(&mut self, value: T);

    /// Terminated
    fn on_terminal(self: Box<Self>, terminal: Terminal<E>);
}
