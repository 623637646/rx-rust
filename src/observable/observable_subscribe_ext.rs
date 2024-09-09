use super::Observable;
use crate::{
    cancellable::Cancellable,
    observer::{anonymous_observer::AnonymousObserver, Event},
};

/// Extension trait for `Observable`
pub trait ObservableSubscribeExt<T, E> {
    /**
    Subscribes to the observable with the given `on_event` callback.

    Example:
    ```rust
    use rx_rust::{
        observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    };
    #[derive(Debug)]
    struct MyStruct {}
    let observable = Just::new(MyStruct {});
    observable.subscribe_on_event(move |event| {
        println!("{:?}", event);
    });
    ```
    */
    fn subscribe_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(Event<&'a T, E>) + 'static;

    /**
    Subscribes to the observable with the given `on_next` callback.

    Example:
    ```rust
    use rx_rust::{
        observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    };
    #[derive(Debug)]
    struct MyStruct {}
    let observable = Just::new(MyStruct {});
    observable.subscribe_on_next(move |value| {
        println!("{:?}", value);
    });
    ```
    */
    fn subscribe_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(&'a T) + 'static;
}

impl<T, E, O> ObservableSubscribeExt<T, E> for O
where
    O: Observable<T, E>,
{
    fn subscribe_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(Event<&'a T, E>) + 'static,
    {
        let observer = AnonymousObserver::new(on_event);
        self.subscribe(observer)
    }

    fn subscribe_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: for<'a> Fn(&'a T) + 'static,
    {
        self.subscribe_on_event(move |event| {
            if let Event::Next(value) = event {
                on_next(value);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::Observer, operators::just::Just, utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_on_event() {
        struct MyStruct {
            value: i32,
        }
        let observable = Just::new(MyStruct { value: 333 });
        let checker = CheckingObserver::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_event(move |event| {
            checker_cloned.on(event.map_next(|my_struct| my_struct.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_on_next() {
        struct MyStruct {
            value: i32,
        }
        let observable = Just::new(MyStruct { value: 333 });
        let checker = CheckingObserver::<i32, String>::new();
        let checker_cloned = checker.clone();
        observable.subscribe_on_next(move |value| {
            checker_cloned.on(Event::Next(value.value));
        });
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_unterminated());
    }
}
