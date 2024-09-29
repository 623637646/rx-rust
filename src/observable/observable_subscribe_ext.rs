use super::Observable;
use crate::{
    observer::{Observer, Terminal},
    subscriber::Subscriber,
};

/// The `ObservableSubscribeExt` trait provides a convenient method to subscribe to an observable
/// with custom `on_next` and `on_terminal` callbacks. This allows for more flexible and ergonomic
/// usage of observables in the code.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `FN` - The type of the callback function for handling emitted items.
/// * `FT` - The type of the callback function for handling terminal events.

pub trait ObservableSubscribeExt<T, E, FN, FT> {
    /// Subscribes to the observable with the given `on_next` and `on_terminal` callbacks.
    ///
    /// # Arguments
    ///
    /// * `on_next` - A callback function that will be called with each item emitted by the observable.
    /// * `on_terminal` - A callback function that will be called when the observable emits a terminal event.
    ///
    /// # Returns
    ///
    /// A `Subscriber` which can be used to unsubscribe the observer.
    ///
    /// # Example
    /// ```rust
    /// use rx_rust::{
    ///     observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    /// };
    /// use std::convert::Infallible;
    /// use rx_rust::observer::Terminal;
    /// let observable = Just::new(123);
    /// observable.subscribe_on(
    ///     move |value: i32| {
    ///         println!("Next value: {}", value);
    ///     },
    ///     move |terminal: Terminal<Infallible>| {
    ///         println!("Terminal event: {:?}", terminal);
    ///     }
    /// );
    /// ```
    fn subscribe_on(self, on_next: FN, on_terminal: FT) -> Subscriber;
}

impl<T, E, FN, FT, OE> ObservableSubscribeExt<T, E, FN, FT> for OE
where
    FN: FnMut(T),
    FT: FnOnce(Terminal<E>),
    OE: Observable<T, E, ObservableSubscribeExtObserver<FN, FT>>,
{
    fn subscribe_on(self, on_next: FN, on_terminal: FT) -> Subscriber {
        let observer = ObservableSubscribeExtObserver {
            on_next,
            on_terminal,
        };
        self.subscribe(observer)
    }
}

/// The `ObservableSubscribeExtObserver` struct is an implementation of the `Observer` trait
/// that allows subscribing to an observable with custom `on_next` and `on_terminal` callbacks.
///
/// # Type Parameters
///
/// * `FN` - The type of the callback function for handling emitted items.
/// * `FT` - The type of the callback function for handling terminal events.
///
/// # Fields
///
/// * `on_next` - A callback function that will be called with each item emitted by the observable.
/// * `on_terminal` - A callback function that will be called when the observable emits a terminal event.

pub struct ObservableSubscribeExtObserver<FN, FT> {
    on_next: FN,
    on_terminal: FT,
}

impl<T, E, FN, FT> Observer<T, E> for ObservableSubscribeExtObserver<FN, FT>
where
    FN: FnMut(T),
    FT: FnOnce(Terminal<E>),
{
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.on_terminal)(terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::Observer, operators::just::Just, utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_subscribe_on() {
        let observable = Just::new(123);
        let checker = CheckingObserver::new();
        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        observable.subscribe_on(
            move |value| {
                checker_cloned_1.on_next(value);
            },
            move |terminal| {
                checker_cloned_2.on_terminal(terminal);
            },
        );
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_completed());
    }
}
