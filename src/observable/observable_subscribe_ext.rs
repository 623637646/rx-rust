use super::Observable;
use crate::{
    observer::{anonymous_observer::AnonymousObserver, event::Event, Observer, Terminal},
    subscription::Subscription,
};

/// Extension trait for `Observable`
pub trait ObservableSubscribeExt<T, E, OR> {
    /**
    Subscribes to the observable with the given `on_event` callback.

    # Example
    ```rust
    use rx_rust::{
        observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    };
    use std::convert::Infallible;
    use rx_rust::observer::event::Event;
    let observable = Just::new(123);
    observable.subscribe_on_event(move |event: Event<i32, Infallible>| {
        println!("{:?}", event);
    });
    ```
    */
    fn subscribe_on(
        self,
        on_next: impl Fn(T),
        on_terminal: impl FnOnce(Terminal<E>),
    ) -> Subscription;

    /**
    Subscribes to the observable with the given `on_next` callback.

    # Example
    ```rust
    use rx_rust::{
        observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    };
    let observable = Just::new(123);
    observable.subscribe_on_next(move |value: i32| {
        println!("{:?}", value);
    });
    ```
    */
    fn subscribe_on_next(self, on_next: impl Fn(T)) -> Subscription;
}

impl<T, E, OR, OE> ObservableSubscribeExt<T, E, OR> for OE
where
    OR: Observer<T, E>,
    OE: Observable<T, E, OR>,
{
    fn subscribe_on(
        self,
        on_next: impl Fn(T),
        on_terminal: impl FnOnce(Terminal<E>),
    ) -> Subscription {
        let observer = AnonymousObserver::new(on_next, on_terminal);
        self.subscribe(observer)
    }

    fn subscribe_on_next(self, on_next: impl Fn(T)) -> Subscription {
        self.subscribe_on(on_next, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::Observer, operators::just::Just, utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_on_event() {
        let observable = Just::new(123);
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.notify_if_unterminated(event);
        });
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_on_next() {
        let observable = Just::new(123);
        let checker = CheckingObserver::<i32, String>::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_next(move |value| {
            checker_cloned.notify_if_unterminated(Event::Next(value));
        });
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_unterminated());
    }
}
