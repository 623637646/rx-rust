pub mod hot_observable;
pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscription::Subscription};

/// An `Observable` can be subscribed to by an `Observer`.
pub trait Observable<'a, T, E> {
    /// Subscribes an observer to this observable. Returns a Subscription that can be unsubscribed.
    /// The Observer must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    fn subscribe(&'a self, observer: impl Observer<T, E> + 'static) -> Subscription;
}
