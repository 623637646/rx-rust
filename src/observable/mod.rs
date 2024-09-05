pub mod hot_observable;
pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::cancellable::Cancellable;
use crate::observer::Observer;

/// An `Observable` is a type that can be subscribed to by an `Observer`.
pub trait Observable<'a, T, E> {
    /// Subscribes an observer to this observable.
    /// Returns a cancellable that can be used to cancel the subscription.
    /// The O type must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
    where
        O: Observer<T, E> + 'static;
}
