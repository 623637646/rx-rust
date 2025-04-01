// pub mod observable_into_ext; // TODO: Do we really need this trait?
pub mod observable_subscribe_ext;

use crate::{observer::Observer, subscription::Subscription};

/// The `Observable` trait represents a source of events that can be observed by an `Observer`.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `OR` - The type of the observer that will receive events from the observable. It must implement the `Observer` trait.
///     We use `OR` generic type instead of this code:
///     ```text
///     pub trait Observable<T, E> {
///         fn subscribe(self, observer: impl Observer<T, E>) -> Subscription;
///     }
///     ```
///     Because `Create` operator (or others) needs the `OR` generic type in the callback function.
pub trait Observable<'a, T, E, OR>
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
    /// struct MyObservable;
    ///
    /// impl<'a> Observable<'a, i32, (), MyObserver> for MyObservable {
    ///     fn subscribe(self, mut observer: MyObserver) -> Subscription<'a> {
    ///         observer.on_next(1);
    ///         observer.on_terminal(Terminal::Completed);
    ///         Subscription::new_none_disposal()
    ///     }
    /// }
    ///
    /// let observable = MyObservable;
    /// let observer = MyObserver;
    /// let subscription = observable.subscribe(observer);
    /// ```
    fn subscribe(self, observer: OR) -> Subscription<'a>;
}
