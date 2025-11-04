pub mod boxed_observable;
pub mod observable_ext;

use crate::{
    disposable::subscription::Subscription, observer::Observer, utils::types::NecessarySendSync,
};

/// The `Observable` trait represents a source of events that can be observed by an `Observer`.
/// See <https://reactivex.io/documentation/observable.html>
pub trait Observable<'or, 'sub, T, E> {
    /// Subscribes an observer to this observable. When an observer is subscribed, it will start receiving events from the observable.
    /// The `subscribe` method returns a `Subscription` which can be used to unsubscribe the observer from the observable.
    /// We use `Subscription` struct instead of trait like `impl Cancellable`, because we need to cancel the subscription when the `Subscription` is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub>;
}
