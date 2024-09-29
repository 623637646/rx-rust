pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscriber::Subscriber};

/// An `Observable` can be subscribed to by an `Observer`.
pub trait Observable<T, E, OR>: Clone
where
    OR: Observer<T, E>,
{
    /// Subscribes an observer to this observable. Returns a Subscriber that can be unsubscribed.
    // It returns Subscriber struct instead of trait like `impl Cancellable`, because we need to cancel the subscriber when the `Subscriber` is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: OR) -> Subscriber;
}
