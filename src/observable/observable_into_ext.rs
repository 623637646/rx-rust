use super::Observable;

/**
This trait is used to convert any type that implements `Observable` into `impl Observable<'_, T, E>`.

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
    /// Converts any type that implements `Observable` into `impl Observable<'_, T, E>`.
    fn into_observable(self) -> impl for<'a> Observable<'a, T, E>;
}

impl<T, E, O> ObservableIntoExt<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
{
    fn into_observable(self) -> impl for<'a> Observable<'a, T, E> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancellable::non_cancellable::NonCancellable;
    use crate::observer::event::{Event, Terminated};
    use crate::observer::Observer;
    use crate::operators::create::Create;
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_normal() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
