use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use std::marker::PhantomData;

/**
This is an observable that emits the values provided by the subscribe_handler function.

# Example
```rust
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::observer::Observer;
use rx_rust::subscription::Subscription;
use rx_rust::operators::create::Create;
use rx_rust::observer::Terminal;
let observable = Create::new(|mut observer| {
    observer.on_next(1);
    observer.on_next(2);
    observer.on_next(3);
    observer.on_terminal(Terminal::Completed);
    Subscription::new_empty()
});
observable.subscribe_on(
    move |value| println!("value: {}", value),
    move |terminal: Terminal<String>| println!("terminal: {:?}", terminal),
);
```
*/
pub struct Create<F, OR> {
    subscribe_handler: F,
    _marker: PhantomData<OR>,
}

// Using `F: FnMut(CreateObserver<OR>) -> Subscription + Clone` to make Create more easy to use. See more in `struct CreateObserver`
impl<F, OR> Create<F, OR>
where
    F: FnMut(CreateObserver<OR>) -> Subscription + Clone,
{
    pub fn new(subscribe_handler: F) -> Create<F, OR> {
        Create {
            subscribe_handler,
            _marker: PhantomData,
        }
    }
}

impl<F, OR> Clone for Create<F, OR>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Create {
            subscribe_handler: self.subscribe_handler.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, E, OR, F> Observable<T, E, OR> for Create<F, OR>
where
    OR: Observer<T, E>,
    F: FnMut(CreateObserver<OR>) -> Subscription + Clone,
{
    fn subscribe(mut self, observer: OR) -> Subscription {
        (self.subscribe_handler)(CreateObserver::new(observer))
    }
}

/** Using `CreateObserver<OR>` instead of directly using `OR` to make `Create` more easy to use.
Other wise, this code can't be compiled:
```rust
use rx_rust::operators::create::Create;
use rx_rust::observer::Terminal;
use rx_rust::subscription::Subscription;
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::observer::Observer;
let observable = Create::new(|mut observer| {
    observer.on_next(1);
    observer.on_terminal(Terminal::<String>::Completed);
    Subscription::new_empty()
});
observable.subscribe_on(
    move |value| {},
    move |terminal| {},
);
```
*/
pub struct CreateObserver<OR>(OR);

impl<OR> CreateObserver<OR> {
    pub(crate) fn new(observer: OR) -> Self {
        Self(observer)
    }
}

impl<T, E, OR> Observer<T, E> for CreateObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.on_next(value)
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.0.on_terminal(terminal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observer::Terminal, utils::checking_observer::CheckingObserver};

    #[test]
    fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_empty()
        });
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(33);
            observer.on_next(44);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_empty()
        });

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[33, 44]));
        assert!(checker.is_error("error"));
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            Subscription::new_empty()
        });

        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_empty()
        });

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
