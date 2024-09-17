pub mod anonymous_observer;
pub mod event;
pub mod observer_ext;

use event::Event;

/// An `Observer` is a type that can receive events from an `Observable`.
/// The observer must be Sync and Send because it will be used in multiple threads. See Scheduler usage in delay.rs.
/// The observer must be 'static because it will be stored in Subscription or hot observables.
pub trait Observer<T, E>: Sync + Send + 'static {
    /// Received an event from an `Observable`.
    fn on(&self, event: Event<T, E>);

    /// Get whether the observer is terminated.
    fn terminated(&self) -> bool;

    /// Set the observer to be terminated.
    fn set_terminated(&self, terminated: bool);

    /// Notify the observer if it is not terminated.
    fn notify_if_unterminated(&self, event: Event<T, E>) {
        if self.terminated() {
            return;
        }
        match event {
            Event::Next(_) => self.on(event),
            Event::Terminated(_) => {
                self.set_terminated(true);
                self.on(event);
            }
        }
    }
}
