use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    utils::{disposal::Disposal, never::Never},
};

/**
This is an observable that emits a single value then completes.

# Example
```rust
use rx_rust::operators::just::Just;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::utils::never::Never;
use rx_rust::observer::event::Event;
let observable = Just::new(123);
observable.subscribe_on_event(|event: Event<i32, Never>| println!("event: {:?}", event));
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

impl<T, O> Observable<T, Never, O> for Just<T>
where
    O: Observer<T, Never>,
    T: Clone + Sync + Send + 'static,
{
    fn subscribe(self, mut observer: O) -> Disposal {
        observer.on_next(self.value.clone());
        Box::new(observer).on_terminal(Terminal::Completed);
        Disposal::new_no_action()
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
