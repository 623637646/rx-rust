/**
Subscription is from Observable pattern, it is used to unsubscribe the observable.
The dispose closure will be called when the subscription is unsubscribed or dropped.
The dispose closure will be called at most once.

# Example
```rust
use rx_rust::subscription::Subscription;
let subscription = Subscription::new(|| println!("Clean up"));
subscription.unsubscribe();
```
*/
pub struct Subscription {
    /// The closure to call when the subscription is unsubscribed or dropped.
    /// None means no need to do anything when unsubscribed.
    dispose: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    pub fn new(dispose: impl FnOnce() + 'static) -> Subscription {
        Subscription {
            dispose: Some(Box::new(dispose) as Box<dyn FnOnce()>),
        }
    }

    pub fn non_dispose() -> Subscription {
        Subscription { dispose: None }
    }

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
    use std::{cell::Cell, rc::Rc};

    use super::*;

    #[test]
    fn test_unsubscribe() {
        let disposed = Rc::new(Cell::new(false));
        let disposed_clone = disposed.clone();
        let subscription = Subscription::new(move || {
            assert!(!disposed.get());
            disposed.set(true);
        });
        subscription.unsubscribe();
        assert!(disposed_clone.get());
    }

    #[test]
    fn test_dropped() {
        let disposed = Rc::new(Cell::new(false));
        let disposed_clone = disposed.clone();
        {
            Subscription::new(move || {
                assert!(!disposed.get());
                disposed.set(true);
            });
        }
        assert!(disposed_clone.get());
    }
}
