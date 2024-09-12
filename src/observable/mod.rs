pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscription::Subscription};

/// An Observable can be subscribed to by an `Observer`.
/// Observable must be Sync and Send because it will be used in multiple threads.
/// Observable must be 'static because it may be stored in somewhere.
pub trait Observable<T, E>: Clone + Sync + Send + 'static {
    /// Subscribes an observer to this observable. Returns a Subscription that can be unsubscribed.
    /// It returns Subscription struct instead of trait like `impl Cancellable`, because we need Disposal to cancel the subscription when the Subscription is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: impl Observer<T, E>) -> Subscription;
}
