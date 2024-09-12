use crate::{
    observer::{
        event::{Event, Terminated},
        Observer,
    },
    utils::disposal::Disposal,
};

/**
Subscription is from Observable pattern, it is used to unsubscribe the observable.

# Example
```rust
use rx_rust::subscription::Subscription;
use rx_rust::observer::event::Event;
use rx_rust::observer::anonymous_observer::AnonymousObserver;
let observer = AnonymousObserver::new(|event: Event<i32, String>| {
    println!("{:?}", event);
});
let subscription = Subscription::new(observer, move || {
    println!("Clean up");
});
subscription.unsubscribe();
```
*/
pub struct Subscription {
    disposal: Disposal<Box<dyn FnOnce() + Sync + Send + 'static>>,
}

impl Subscription {
    /// Create a new Subscription with the observer and disposal_action.
    /// The observer will be notified with Event::Terminated(Terminated::Unsubscribed) if it's unterminated when the subscription is unsubscribed or dropped.
    /// The disposal_action will be called when the subscription is unsubscribed or dropped.
    pub fn new<T, E, O, F>(observer: O, disposal_action: F) -> Subscription
    where
        O: Observer<T, E>,
        F: FnOnce() + Sync + Send + 'static,
    {
        Subscription {
            disposal: Disposal::new(Box::new(move || {
                disposal_action();
                if !observer.terminated() {
                    observer.notify_if_unterminated(Event::Terminated(Terminated::Unsubscribed));
                }
            })),
        }
    }

    /// Create a new Subscription with the observer.
    /// The observer will be notified with Event::Terminated(Terminated::Unsubscribed) if it's unterminated when the subscription is unsubscribed or dropped.
    pub fn new_non_disposal_action<T, E, O>(observer: O) -> Subscription
    where
        O: Observer<T, E>,
    {
        Subscription {
            disposal: Disposal::new(Box::new(move || {
                if !observer.terminated() {
                    observer.notify_if_unterminated(Event::Terminated(Terminated::Unsubscribed));
                }
            })),
        }
    }

    /// Unsubscribe the subscription.
    pub fn unsubscribe(self) {
        self.disposal.dispose();
    }

    /// Insert a disposal action before the original disposal action.
    pub fn insert_disposal_action<F>(self, disposal_action: F) -> Self
    where
        F: FnOnce() + Sync + Send + 'static,
    {
        let original_disposal = self.disposal;
        Subscription {
            disposal: Disposal::new(Box::new(move || {
                disposal_action();
                original_disposal.dispose();
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::checking_observer::CheckingObserver;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_unsubscribe_with_action() {
        let checker = CheckingObserver::<i32, String>::new();
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        let subscription = Subscription::new(checker.clone(), move || {
            let mut disposed = disposed_clone.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        assert!(!*disposed.read().unwrap());
        checker.is_values_matched(&[]);
        checker.is_unterminated();
        subscription.unsubscribe();
        assert!(*disposed.read().unwrap());
        checker.is_values_matched(&[]);
        checker.is_unsubscribed();
    }

    #[test]
    fn test_dropped_with_action() {
        let checker = CheckingObserver::<i32, String>::new();
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        {
            let subscription = Subscription::new(checker.clone(), move || {
                let mut disposed = disposed_clone.write().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
            assert!(!*disposed.read().unwrap());
            checker.is_values_matched(&[]);
            checker.is_unterminated();
            _ = subscription; // keep the subscription alive
        }
        assert!(*disposed.read().unwrap());
        checker.is_values_matched(&[]);
        checker.is_unsubscribed();
    }

    #[test]
    fn test_unsubscribe_without_action() {
        let checker = CheckingObserver::<i32, String>::new();
        let subscription = Subscription::new_non_disposal_action(checker.clone());
        checker.is_values_matched(&[]);
        checker.is_unterminated();
        subscription.unsubscribe();
        checker.is_values_matched(&[]);
        checker.is_unsubscribed();
    }

    #[test]
    fn test_dropped_without_action() {
        let checker = CheckingObserver::<i32, String>::new();
        {
            let subscription = Subscription::new_non_disposal_action(checker.clone());
            checker.is_values_matched(&[]);
            checker.is_unterminated();
            _ = subscription; // keep the subscription alive
        }
        checker.is_values_matched(&[]);
        checker.is_unsubscribed();
    }

    #[test]
    fn test_insert_disposal_action() {
        let checker = CheckingObserver::<i32, String>::new();
        let counter = Arc::new(RwLock::new(0));
        let counter_cloned_1 = counter.clone();
        let counter_cloned_2 = counter.clone();
        let subscription = Subscription::new(checker.clone(), move || {
            let mut counter = counter_cloned_1.write().unwrap();
            assert!(*counter == 1);
            *counter = 2;
        });
        assert!(*counter.read().unwrap() == 0);
        checker.is_values_matched(&[]);
        checker.is_unterminated();
        let subscription = subscription.insert_disposal_action(move || {
            let mut counter = counter_cloned_2.write().unwrap();
            assert!(*counter == 0);
            *counter = 1;
        });
        assert!(*counter.read().unwrap() == 0);
        checker.is_values_matched(&[]);
        checker.is_unterminated();

        subscription.unsubscribe();
        assert!(*counter.read().unwrap() == 2);
        checker.is_values_matched(&[]);
        checker.is_unsubscribed();
    }
}
