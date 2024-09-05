pub mod hot_observable;
pub mod observable_ext;

use crate::cancellable::Cancellable;
use crate::observer::Observer;

/// An `Observable` is a type that can be subscribed to by an `Observer`.
pub trait Observable<'a, T, E> {
    /// Subscribes an observer to this observable.
    /// Returns a cancellable that can be used to cancel the subscription.
    fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
    where
        O: Observer<T, E> + 'static;
}
