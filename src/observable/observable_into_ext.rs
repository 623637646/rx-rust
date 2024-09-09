use super::Observable;

/**
This trait is used to convert any type that implements `Observable` into `impl Observable<T, E>`.

# Example
```rust
use rx_rust::{
    observable::{
        observable_into_ext::ObservableIntoExt,
        observable_subscribe_ext::ObservableSubscribeExt,
    },
    operators::just::Just,
};
let just = Just::new(123);
let observable = just.into_observable();
observable.subscribe_on_next(|value| {
    println!("value: {}", value);
});
```
 */

pub trait ObservableIntoExt<T, E> {
    /// Converts any type that implements `Observable` into `impl Observable<T, E>`.
    fn into_observable(self) -> impl Observable<T, E>;
}

impl<T, E, O> ObservableIntoExt<T, E> for O
where
    O: Observable<T, E>,
{
    fn into_observable(self) -> impl Observable<T, E> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancellable::non_cancellable::NonCancellable;
    use crate::observable::observable_cloned::ObservableCloned;
    use crate::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use crate::observer::{Event, Observer, Terminated};
    use crate::operators::create::Create;
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_ref() {
        struct MyStruct {
            value: i32,
        }
        let observable = Create::new(
            |observer: Box<dyn for<'a> Observer<&'a MyStruct, String>>| {
                let my_struct = MyStruct { value: 333 };
                observer.on(Event::Next(&my_struct));
                observer.on(Event::Terminated(Terminated::Completed));
                NonCancellable
            },
        );
        let observable = observable.into_observable();
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
        let observable = Create::new(|observer: Box<dyn for<'a> Observer<&'a i32, String>>| {
            observer.on(Event::Next(&333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_ref_multiple() {
        struct MyStruct {
            value: i32,
        }
        let observable = Create::new(
            |observer: Box<dyn for<'a> Observer<&'a MyStruct, String>>| {
                let my_struct = MyStruct { value: 333 };
                observer.on(Event::Next(&my_struct));
                observer.on(Event::Terminated(Terminated::Completed));
                NonCancellable
            },
        );
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.on(event.map_value(|my_struct| my_struct.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_cloned_multiple() {
        let observable = Create::new(|observer: Box<dyn for<'a> Observer<&'a i32, String>>| {
            observer.on(Event::Next(&333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
