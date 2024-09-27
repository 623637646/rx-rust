use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use std::sync::Arc;

/**
This is an observable that emits the values provided by the subscribe_handler function.

# Example
```rust
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::observer::event::Event;
use rx_rust::observer::Observer;
use rx_rust::subscription::Subscription;
use rx_rust::observer::event::Terminated;
use rx_rust::operators::create::Create;
let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    observer.notify_if_unterminated(Event::Next(1));
    observer.notify_if_unterminated(Event::Next(2));
    observer.notify_if_unterminated(Event::Next(3));
    observer.notify_if_unterminated(Event::Terminated(Terminated::Completed));
    Subscription::new_non_disposal_action(observer)
});
observable.subscribe_on_event(|event: Event<i32, String>| println!("event: {:?}", event));
```
*/
pub struct Create<F> {
    subscribe_handler: Arc<F>,
}

impl<F> Create<F> {
    pub fn new(subscribe_handler: F) -> Create<F> {
        Create {
            subscribe_handler: Arc::new(subscribe_handler),
        }
    }
}

impl<F> Clone for Create<F> {
    fn clone(&self) -> Self {
        Create {
            subscribe_handler: self.subscribe_handler.clone(),
        }
    }
}

impl<T, E, OR, F> Observable<T, E, OR> for Create<F>
where
    OR: Observer<T, E>,
    F: Fn(OR) -> Subscription,
{
    fn subscribe(self, observer: OR) -> Subscription {
        (self.subscribe_handler)(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observer::Terminal, utils::checking_observer::CheckingObserver};

    #[test]
    fn test_completed() {
        let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
            observer.on_next(333);
            observer.on_terminal(Terminal::Completed);
            Subscription::new_non_disposal_action()
        });
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer: CheckingObserver<i32, &'static str>| {
            observer.on_next(33);
            observer.on_next(44);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_non_disposal_action()
        });

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[33, 44]));
        assert!(checker.is_error("error"));
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
            observer.on_next(1);
            Subscription::new_non_disposal_action()
        });

        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
            observer.on_next(333);
            observer.on_terminal(Terminal::Completed);
            Subscription::new_non_disposal_action()
        });

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
