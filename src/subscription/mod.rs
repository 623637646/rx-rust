/**
Subscription is from Observable pattern, it is used to unsubscribe the observable.

# Example
```rust
use rx_rust::subscription::Subscription;
let subscription = Subscription::new(move || {
    println!("Clean up");
});
subscription.unsubscribe();
```
*/
pub struct Subscription {
    dispose: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    /// Create a new Subscription with a disposal action.
    /// The dispose will be called when the subscription is unsubscribed or dropped.
    pub fn new<F>(dispose: F) -> Subscription
    where
        F: FnOnce() + 'static,
    {
        Subscription {
            dispose: Some(Box::new(dispose)),
        }
    }

    /// Create a new empty Subscription. No action will be performed when the subscription is unsubscribed or dropped.
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
        if let Some(dispose) = self.dispose.take() {
            dispose();
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
