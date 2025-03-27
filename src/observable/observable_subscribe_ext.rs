use super::Observable;
use crate::{
    observer::{Observer, Terminal},
    subscription::Subscription,
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
pub trait ObservableSubscribeExt<'a, T, E> {
    /// Subscribes to the observable with the given `on_next` and `on_terminal` callbacks.
    ///
    /// # Arguments
    ///
    /// * `on_next` - A callback function that will be called with each item emitted by the observable.
    /// * `on_terminal` - A callback function that will be called when the observable emits a terminal event.
    ///
    /// # Returns
    ///
    /// A `Subscription` which can be used to unsubscribe the observer.
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
    ///     |value| {
    ///         println!("Next value: {}", value);
    ///     },
    ///     |terminal| {
    ///         println!("Terminal event: {:?}", terminal);
    ///     }
    /// );
    /// ```
    fn subscribe_on<FN, FT>(self, on_next: FN, on_terminal: FT) -> Subscription<'a>
    where
        FN: FnMut(T) + Send + 'a,
        FT: FnOnce(Terminal<E>) + Send + 'a;
}

impl<'a, T, E, OE> ObservableSubscribeExt<'a, T, E> for OE
where
    OE: Observable<'a, T, E, ObservableSubscribeExtObserver<'a, T, E>>,
{
    fn subscribe_on<FN, FT>(self, on_next: FN, on_terminal: FT) -> Subscription<'a>
    where
        FN: FnMut(T) + Send + 'a,
        FT: FnOnce(Terminal<E>) + Send + 'a,
    {
        let observer = ObservableSubscribeExtObserver {
            on_next: Box::new(on_next),
            on_terminal: Box::new(on_terminal),
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
pub struct ObservableSubscribeExtObserver<'a, T, E> {
    on_next: Box<dyn FnMut(T) + Send + 'a>,
    on_terminal: Box<dyn FnOnce(Terminal<E>) + Send + 'a>,
}

impl<T, E> Observer<T, E> for ObservableSubscribeExtObserver<'_, T, E> {
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
        observer::Observer,
        operators::{create::Create, just::Just},
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_subscribe_on() {
        let observable = Just::new(123);
        let checker = CheckingObserver::new();
        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        observable.subscribe_on(
            |value| {
                checker_cloned_1.on_next(value);
            },
            |terminal| {
                checker_cloned_2.on_terminal(terminal);
            },
        );
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_on_twice() {
        // Use `Create` to test the `subscribe_on` method twice.
        // `Create` is a special in this case to avoid compile error "no two closures, even if identical, have the same type".
        // Check the commit of this code change for more details.
        let observable = Create::new(|mut observer| {
            observer.on_next(123);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });
        let checker1 = CheckingObserver::new();
        let checker2 = CheckingObserver::new();
        let mut checker_cloned_1_1 = checker1.clone();
        let checker_cloned_1_2 = checker1.clone();
        observable.clone().subscribe_on(
            |value| {
                checker_cloned_1_1.on_next(value);
            },
            |terminal| {
                checker_cloned_1_2.on_terminal(terminal);
            },
        );

        let mut checker_cloned_2_1 = checker2.clone();
        let checker_cloned_2_2 = checker2.clone();
        observable.subscribe_on(
            |value| {
                checker_cloned_2_1.on_next(value);
            },
            |terminal| {
                checker_cloned_2_2.on_terminal(terminal);
            },
        );

        assert!(checker1.is_values_matched(&[123]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[123]));
        assert!(checker2.is_completed());
    }
}
