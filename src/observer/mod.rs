pub mod anonymous_observer;
pub mod event;

use event::Event;

/// An `Observer` is a type that can receive events from an `Observable`.
pub trait Observer<T, E> {
    fn on(&self, event: Event<T, E>);
}
