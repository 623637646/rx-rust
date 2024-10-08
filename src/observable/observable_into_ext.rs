use super::Observable;

/**
This trait is used to convert any type that implements `Observable` into `impl Observable<T, E>`.

# Example
```rust
use rx_rust::{
    observable::{
        observable_into_ext::ObservableIntoExt,
        observable_subscribe_ext::ObservableSubscribeExt,
    },
    operators::just::Just,
};
let observable = Just::new(123);
let observable = observable.into_observable();
observable.subscribe_on_next(|value| {
    println!("value: {}", value);
});
```
 */

pub trait ObservableIntoExt<T, E> {
    /// Converts any type that implements `Observable` into `impl Observable<T, E>`.
    fn into_observable(self) -> impl Observable<T, E>;
}

impl<T, E, O> ObservableIntoExt<T, E> for O
where
    O: Observable<T, E>,
{
    fn into_observable(self) -> impl Observable<T, E> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::event::{Event, Terminated};
    use crate::observer::Observer;
    use crate::operators::create::Create;
    use crate::subscription::Subscription;
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_normal() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.notify_if_unterminated(Event::Next(333));
            observer.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.notify_if_unterminated(Event::Next(333));
            observer.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
