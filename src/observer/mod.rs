pub mod anonymous_observer;
pub mod event;
pub mod observer_ext;

use event::Event;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Terminal<E> {
    Completed,
    Error(E),
}

/// An `Observer` is a type that can receive events from an `Observable`.
/// The observer must be Sync and Send because it will be used in multiple threads. See Scheduler usage in delay.rs.
/// The observer must be 'static because it will be stored in Subscription or hot observables.
pub trait Observer<T, E> {
    fn on_next(&mut self, value: T);

    fn on_terminal(self, terminal: Terminal<E>);
}
