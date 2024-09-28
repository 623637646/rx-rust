pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscription::Subscription};

/// An `Observable` can be subscribed to by an `Observer`.
pub trait Observable<T, E, OR>: Clone
where
    OR: Observer<T, E>,
{
    /// Subscribes an observer to this observable. Returns a Subscription that can be unsubscribed.
    /// It returns Subscription struct instead of trait like `impl Cancellable`, because we need to cancel the subscription when the `Subscription` is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: OR) -> Subscription;
}
