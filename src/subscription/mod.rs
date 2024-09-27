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
    dispose: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    /// Create a new Subscription with the observer and disposal_action.
    /// The observer will be notified with Event::Terminated(Terminated::Unsubscribed) if it's unterminated when the subscription is unsubscribed or dropped.
    /// The disposal_action will be called when the subscription is unsubscribed or dropped.
    pub fn new<F>(dispose: F) -> Subscription
    where
        F: FnOnce() + 'static,
    {
        Subscription {
            dispose: Some(Box::new(dispose)),
        }
    }

    /// Create a new Subscription with the observer.
    /// The observer will be notified with Event::Terminated(Terminated::Unsubscribed) if it's unterminated when the subscription is unsubscribed or dropped.
    pub fn new_empty() -> Subscription {
        Subscription { dispose: None }
    }

    /// Unsubscribe the subscription.
    pub fn unsubscribe(self) {
        // drop self to call the dispose
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // This code causes tarpaulin wrong coverage report. For more details, see: https://github.com/xd009642/tarpaulin/issues/1624
        // if let Some(action) = self.action.take() {
        //     action();
        // }
        // This code is a workaround for the issue above.
        match self.dispose.take() {
            Some(dispose) => dispose(),
            None => {
                // Do nothing, using this pattern to avoid clippy warning.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_unsubscribe() {
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        let subscription = Subscription::new(move || {
            let mut disposed = disposed_clone.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        assert!(!*disposed.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_dropped() {
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        {
            let subscription = Subscription::new(move || {
                let mut disposed = disposed_clone.write().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
            assert!(!*disposed.read().unwrap());

            _ = subscription; // keep the subscription alive
        }
        assert!(*disposed.read().unwrap());
    }
}
