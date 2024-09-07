pub mod hot_observable;
pub mod observable_cloned;
pub mod observable_into_ext;
pub mod observable_subscribe_cloned_ext;
pub mod observable_subscribe_ext;

use crate::cancellable::Cancellable;
use crate::observer::Observer;

/// An `Observable` is a type that can be subscribed to by an `Observer`. The `Observer` will receive borrowed values.
pub trait Observable<T, E> {
    /// Subscribes an observer to this observable. Returns a cancellable that can be used to cancel the subscription.
    /// The Observer must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    /// The Cancellable must be 'static because it may be stored by callers.
    fn subscribe(
        &self,
        observer: impl for<'a> Observer<&'a T, E> + 'static,
    ) -> impl Cancellable + 'static;
}
