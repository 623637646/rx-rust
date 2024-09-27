use crate::{
    observable::Observable,
    observer::{
        event::{Event, Terminated},
        Observer,
    },
    subscription::Subscription,
};
use std::convert::Infallible;

/**
This is an observable that emits a single value then completes.

# Example
```rust
use rx_rust::operators::just::Just;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use std::convert::Infallible;
use rx_rust::observer::event::Event;
let observable = Just::new(123);
observable.subscribe_on_event(|event: Event<i32, Infallible>| println!("event: {:?}", event));
```
 */

#[derive(Clone)]
pub struct Just<T> {
    value: T,
}

impl<T> Just<T> {
    pub fn new(value: T) -> Just<T> {
        Just { value }
    }
}

impl<T> Observable<T, Infallible> for Just<T>
where
    T: Clone + Sync + Send + 'static,
{
    fn subscribe(self, observer: impl Observer<T, Infallible>) -> Subscription {
        observer.notify_if_unterminated(Event::Next(self.value.clone()));
        observer.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        Subscription::new_non_disposal_action(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_completed() {
        let observable = Just::new(333);
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Just::new(333);

        let checker = CheckingObserver::new();
        observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
