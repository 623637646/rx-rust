/**
Subscriber is from Observable pattern, it is used to unsubscribe the observable.

# Example
```rust
use rx_rust::subscriber::Subscriber;
let subscriber = Subscriber::new(move || {
    println!("Clean up");
});
subscriber.unsubscribe();
```
*/
pub struct Subscriber {
    dispose: Option<Box<dyn FnOnce()>>,
}

impl Subscriber {
    /// Create a new Subscriber with a disposal action.
    /// The dispose will be called when the subscriber is unsubscribed or dropped.
    pub fn new<F>(dispose: F) -> Subscriber
    where
        F: FnOnce() + 'static,
    {
        Subscriber {
            dispose: Some(Box::new(dispose)),
        }
    }

    /// Create a new empty Subscriber. No action will be performed when the subscriber is unsubscribed or dropped.
    pub fn new_empty() -> Subscriber {
        Subscriber { dispose: None }
    }

    /// Unsubscribe the subscriber.
    pub fn unsubscribe(self) {
        // drop self to call the dispose
    }
}

impl Drop for Subscriber {
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
        let subscriber = Subscriber::new(move || {
            let mut disposed = disposed_clone.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        assert!(!*disposed.read().unwrap());
        subscriber.unsubscribe();
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_dropped() {
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        {
            let subscriber = Subscriber::new(move || {
                let mut disposed = disposed_clone.write().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
            assert!(!*disposed.read().unwrap());

            _ = subscriber; // keep the subscriber alive
        }
        assert!(*disposed.read().unwrap());
    }
}
