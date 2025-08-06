pub mod boxed_observable;
pub mod observable_ext;

use crate::{
    disposable::subscription::Subscription, observer::Observer, utils::types::NecessarySend,
};

/// The `Observable` trait represents a source of events that can be observed by an `Observer`.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `OR` - The type of the observer that will receive events from the observable. It must implement the `Observer` trait.
///   We use `OR` generic type instead of this code:
///   ```text
///   pub trait Observable<T, E> {
///       fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription;
///   }
///   ```
///   Because `Create` operator (or others) needs the `OR` generic type in the callback function.
pub trait Observable<'or, 'sub, T, E> {
    /// Subscribes an observer to this observable.
    ///
    /// When an observer is subscribed, it will start receiving events from the observable.
    /// The `subscribe` method returns a `Subscription` which can be used to unsubscribe the observer
    /// from the observable.
    ///
    /// # Arguments
    ///
    /// * `observer` - The observer that will receive events from this observable.
    ///
    /// # Returns
    ///
    /// A `Subscription` which can be used to unsubscribe the observer.
    /// We use `Subscription` struct instead of trait like `impl Cancellable`, because we need to cancel the subscription when the `Subscription` is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub>;
}
