pub mod observable_into_ext;
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscription::Subscription};

/// The `Observable` trait represents a source of events that can be observed by an `Observer`.
///
/// This trait defines the core functionality of an observable, which is the ability to subscribe an observer to receive events. When an observer is subscribed, it will start receiving events from the observable. The `subscribe` method returns a `Subscription` which can be used to unsubscribe the observer from the observable.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `OR` - The type of the observer that will receive events from the observable. It must implement the `Observer` trait.
pub trait Observable<T, E, OR>
where
    OR: Observer<T, E>,
{
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use rx_rust::observable::Observable;
    /// use rx_rust::observer::{Observer, Terminal};
    /// use rx_rust::subscription::Subscription;
    ///
    /// struct MyObserver;
    ///
    /// impl Observer<i32, ()> for MyObserver {
    ///     fn on_next(&mut self, value: i32) {
    ///         println!("Received value: {}", value);
    ///     }
    ///
    ///     fn on_terminal(self, terminal: Terminal<()>) {
    ///         println!("Terminal: {:?}", terminal);
    ///     }
    /// }
    ///
    /// #[derive(Clone)]
    /// struct MyObservable;
    ///
    /// impl Observable<i32, (), MyObserver> for MyObservable {
    ///     fn subscribe(self, observer: MyObserver) -> Subscription {
    ///         // Implementation here
    ///         Subscription::new_none_disposal()
    ///     }
    /// }
    ///
    /// let observable = MyObservable;
    /// let observer = MyObserver;
    /// let subscription = observable.subscribe(observer);
    /// ```
    fn subscribe(self, observer: OR) -> Subscription;
}
