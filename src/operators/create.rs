use crate::{cancellable::Cancellable, observable::Observable, observer::Observer};

/**
This is an observable that emits the values provided by the subscribe_handler function.

# Example
```rust
use rx_rust::cancellable::non_cancellable::NonCancellable;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::observer::Event;
use rx_rust::observer::Observer;
use rx_rust::observer::Terminated;
use rx_rust::operators::create::Create;
let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    observer.on(Event::Next(1));
    observer.on(Event::Next(2));
    observer.on(Event::Next(3));
    observer.on(Event::Terminated(Terminated::Completed));
    NonCancellable
});
observable.subscribe_on_event(|event: Event<i32, String>| println!("event: {:?}", event));
```
*/
pub struct Create<F> {
    subscribe_handler: F,
}

impl<F> Create<F> {
    pub fn new(subscribe_handler: F) -> Create<F> {
        Create { subscribe_handler }
    }
}

impl<'a, T, E, C, F> Observable<'a, T, E> for Create<F>
where
    C: Cancellable + 'static,
    F: Fn(Box<dyn Observer<T, E>>) -> C,
{
    fn subscribe(&'a self, observer: impl Observer<T, E> + 'static) -> impl Cancellable + 'static {
        (self.subscribe_handler)(Box::new(observer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cancellable::{
            anonymous_cancellable::AnonymousCancellable, non_cancellable::NonCancellable,
        },
        observer::{Event, Terminated},
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_normal() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(333));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(1));
            NonCancellable
        });

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, &str>>| {
            observer.on(Event::Next(33));
            observer.on(Event::Next(44));
            observer.on(Event::Terminated(Terminated::Error("error")));
            NonCancellable
        });

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[33, 44]));
        assert!(checker.is_error("error"));
    }

    #[test]
    fn test_cancelled() {
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        {
            let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
                observer.on(Event::Next(1));
                observer.on(Event::Next(2));
                AnonymousCancellable::new(move || {
                    observer.on(Event::Terminated(Terminated::Cancelled))
                })
            });
            observable.subscribe(checker.clone());
        }
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_cancelled());
    }
}
