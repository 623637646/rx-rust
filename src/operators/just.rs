use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscriber::Subscriber,
};
use std::convert::Infallible;

/**
This is an observable that emits a single value then completes.

# Example
```rust
use rx_rust::operators::just::Just;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use std::convert::Infallible;
use rx_rust::observer::Terminal;
let observable = Just::new(123);
observable.subscribe_on(
    |value| println!("Next value: {}", value),
    |terminal: Terminal<Infallible>| println!("Terminal event: {:?}", terminal)
);
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

impl<T, OR> Observable<T, Infallible, OR> for Just<T>
where
    T: Clone,
    OR: Observer<T, Infallible>,
{
    fn subscribe(self, mut observer: OR) -> Subscriber {
        observer.on_next(self.value.clone());
        observer.on_terminal(Terminal::Completed);
        Subscriber::new_empty()
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
