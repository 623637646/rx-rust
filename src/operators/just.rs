use crate::{
    observable::Observable,
    observer::{
        event::{Event, Terminated},
        Observer,
    },
    subscription::Subscription,
    utils::never::Never,
};

/**
This is an observable that emits a single value then completes.

# Example
```rust
use rx_rust::operators::just::Just;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::utils::never::Never;
use rx_rust::observer::event::Event;
let observable = Just::new(123);
observable.subscribe_on_event(|event: Event<i32, Never>| println!("event: {:?}", event));
```
 */
pub struct Just<T> {
    value: T,
}

impl<T> Just<T> {
    pub fn new(value: T) -> Just<T> {
        Just { value }
    }
}

impl<'a, T> Observable<'a, T, Never> for Just<T>
where
    T: Clone,
{
    fn subscribe(&'a self, observer: impl Observer<T, Never> + 'static) -> Subscription {
        observer.on(Event::Next(self.value.clone()));
        observer.on(Event::Terminated(Terminated::Completed));
        Subscription::non_dispose()
    }
}

impl<'a, T> Observable<'a, &'a T, Never> for Just<T> {
    fn subscribe(&'a self, observer: impl Observer<&'a T, Never> + 'static) -> Subscription {
        observer.on(Event::Next(&self.value));
        observer.on(Event::Terminated(Terminated::Completed));
        Subscription::non_dispose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_normal() {
        let observable = Just::new(333);
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_ref() {
        let observable = Just::new(333);
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event: Event<&i32, Never>| {
            checker_cloned.on(event.map_value(|value| *value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Just::new(333);

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
