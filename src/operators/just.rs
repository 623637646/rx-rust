use crate::{
    cancellable::{non_cancellable::NonCancellable, Cancellable},
    observable::Observable,
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

/**
Create an observable that emits a single value then completes.

# Example
```rust
use rx_rust::operators::just::Just;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
let observable = Just::new(123);
observable.subscribe_on_event(|v| println!("event: {:?}", v));
```
 */
pub struct Just<T> {
    value: T,
}

impl<T> Just<T> {
    pub fn new(value: T) -> Just<T> {
        Just { value }
    }
}

impl<T> Observable<T, Never> for Just<T> {
    fn subscribe(
        &self,
        observer: impl for<'a> Observer<&'a T, Never> + 'static,
    ) -> impl Cancellable + 'static {
        observer.on(Event::Next(&self.value));
        observer.on(Event::Terminated(Terminated::Completed));
        NonCancellable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::{
            observable_cloned::ObservableCloned, observable_subscribe_ext::ObservableSubscribeExt,
        },
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_ref() {
        struct MyStruct {
            value: i32,
        }
        let observable = Just::new(MyStruct { value: 333 });
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.on(event.map_value(|my_struct| my_struct.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_cloned() {
        let observable = Just::new(333);
        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_ref_multiple_subscribe() {
        struct MyStruct {
            value: i32,
        }
        let observable = Just::new(MyStruct { value: 333 });
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.on(event.map_value(|my_struct| my_struct.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.on(event.map_value(|my_struct| my_struct.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_cloned_multiple_subscribe() {
        let observable = Just::new(333);

        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
