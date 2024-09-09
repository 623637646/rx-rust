use crate::cancellable::Cancellable;

/**
Create AnonymousCancellable with a closure.
The closure will be called when the cancellable is cancelled or dropped.
The closure will be called at most once.

Example:
```rust
use rx_rust::cancellable::anonymous_cancellable::AnonymousCancellable;
use rx_rust::cancellable::Cancellable;
let cancellable = AnonymousCancellable::new(|| println!("cancelled"));
cancellable.cancel();
```
*/

pub struct AnonymousCancellable<F>
where
    F: FnOnce(),
{
    action: Option<F>, // None if it's cancelled
}

impl<F> AnonymousCancellable<F>
where
    F: FnOnce(),
{
    pub fn new(action: F) -> AnonymousCancellable<F> {
        AnonymousCancellable {
            action: Some(action),
        }
    }
}

impl<F> Cancellable for AnonymousCancellable<F>
where
    F: FnOnce(),
{
    fn cancel(mut self) {
        let action = self.action.take().unwrap();
        action();
    }
}

impl<F> Drop for AnonymousCancellable<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel() {
        let mut cancelled = false;
        let cancellable = AnonymousCancellable::new(|| {
            assert!(!cancelled);
            cancelled = true;
        });
        cancellable.cancel();
        assert!(cancelled);
    }

    #[test]
    fn test_dropped() {
        let mut cancelled = false;
        {
            AnonymousCancellable::new(|| {
                assert!(!cancelled);
                cancelled = true;
            });
        }
        assert!(cancelled);
    }
}
