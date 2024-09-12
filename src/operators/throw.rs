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
This is an observable that emits an error.

# Example
```rust
use rx_rust::operators::throw::Throw;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::utils::never::Never;
use rx_rust::observer::event::Event;
let observable = Throw::new("My error");
observable.subscribe_on_event(|event: Event<Never, &str>| println!("event: {:?}", event));
```
 */

#[derive(Clone)]
pub struct Throw<E> {
    error: E,
}

impl<E> Throw<E> {
    pub fn new(error: E) -> Throw<E> {
        Throw { error }
    }
}

impl<E> Observable<Never, E> for Throw<E>
where
    E: Clone + Sync + Send + 'static,
{
    fn subscribe(self, observer: impl Observer<Never, E>) -> Subscription {
        observer.notify_if_unterminated(Event::Terminated(Terminated::Error(self.error.clone())));
        Subscription::new_non_disposal_action(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_error() {
        let observable = Throw::new(333);
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(333));
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Throw::new("My error");

        let checker = CheckingObserver::new();
        observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error("My error"));

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error("My error"));
    }
}
