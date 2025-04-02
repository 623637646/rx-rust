use super::Observable;
use crate::{
    observer::{Observer, Terminal},
    subscription::Subscription,
};

/// The `ObservableSubscribeExt` trait provides a convenient method to subscribe to an observable
/// with custom `on_next` and `on_terminal` callbacks. This allows for more flexible and ergonomic
/// usage of observables in the code.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `FN` - The type of the callback function for handling emitted items.
/// * `FT` - The type of the callback function for handling terminal events.
pub trait ObservableSubscribeExt<'a, 'b, T, E> {
    /// Subscribes to the observable with the given `on_next` and `on_terminal` callbacks.
    ///
    /// # Arguments
    ///
    /// * `on_next` - A callback function that will be called with each item emitted by the observable.
    /// * `on_terminal` - A callback function that will be called when the observable emits a terminal event.
    ///
    /// # Returns
    ///
    /// A `Subscription` which can be used to unsubscribe the observer.
    ///
    /// # Example
    /// ```rust
    /// use rx_rust::{
    ///     observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
    /// };
    /// use std::convert::Infallible;
    /// use rx_rust::observer::Terminal;
    /// let observable = Just::new(123);
    /// observable.subscribe_on(
    ///     |value| {
    ///         println!("Next value: {}", value);
    ///     },
    ///     |terminal| {
    ///         println!("Terminal event: {:?}", terminal);
    ///     }
    /// );
    /// ```
    fn subscribe_on<FN, FT>(self, on_next: FN, on_terminal: FT) -> Subscription<'a>
    where
        FN: FnMut(T) + Send + 'b,
        FT: FnOnce(Terminal<E>) + Send + 'b;
}

impl<'a, 'b, T, E, OE> ObservableSubscribeExt<'a, 'b, T, E> for OE
where
    OE: Observable<'a, T, E, ObservableSubscribeExtObserver<'b, T, E>>,
{
    fn subscribe_on<FN, FT>(self, on_next: FN, on_terminal: FT) -> Subscription<'a>
    where
        FN: FnMut(T) + Send + 'b,
        FT: FnOnce(Terminal<E>) + Send + 'b,
    {
        let observer = ObservableSubscribeExtObserver {
            on_next: Box::new(on_next),
            on_terminal: Box::new(on_terminal),
        };
        self.subscribe(observer)
    }
}

/// The `ObservableSubscribeExtObserver` struct is an implementation of the `Observer` trait
/// that allows subscribing to an observable with custom `on_next` and `on_terminal` callbacks.
///
/// # Type Parameters
///
/// * `FN` - The type of the callback function for handling emitted items.
/// * `FT` - The type of the callback function for handling terminal events.
///
/// # Fields
///
/// * `on_next` - A callback function that will be called with each item emitted by the observable.
/// * `on_terminal` - A callback function that will be called when the observable emits a terminal event.
pub struct ObservableSubscribeExtObserver<'a, T, E> {
    on_next: Box<dyn FnMut(T) + Send + 'a>,
    on_terminal: Box<dyn FnOnce(Terminal<E>) + Send + 'a>,
}

impl<T, E> Observer<T, E> for ObservableSubscribeExtObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.on_terminal)(terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::Observer, operators::just::Just, subject::publish_subject::PublishSubject,
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let (on_next, on_terminal) = checker.fn_for_subscribe_on();
        let subscription = observable.subscribe_on(on_next, on_terminal);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let (on_next, on_terminal) = checker.fn_for_subscribe_on();
        let subscription = observable.subscribe_on(on_next, on_terminal);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (on_next, on_terminal) = checker_1.fn_for_subscribe_on();
        let subscription_1 = observable_1.subscribe_on(on_next, on_terminal);
        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_error("error"));

        drop(subscription_2); // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let (on_next, on_terminal) = checker.fn_for_subscribe_on();
        let subscription = observable.subscribe_on(on_next, on_terminal);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let observable = Just::new(&mut value);
        let checker = CheckingObserver::new();

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_on(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| checker_cloned_2.on_terminal(terminal),
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        assert_eq!(value, 222);

        drop(subscription); // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move {
            let (on_next, on_terminal) = checker_cloned.fn_for_subscribe_on();
            observable.subscribe_on(on_next, on_terminal)
        });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let (on_next, on_terminal) = checker_1.fn_for_subscribe_on();
        let subscription_1 = observable.clone().subscribe_on(on_next, on_terminal);
        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable.subscribe_on(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));

        drop(subscription_1); // keep the subscription alive
        drop(subscription_2); // keep the subscription alive
    }

    // Test `on_next` and `on_terminal` with lifetime. See more for the git commit.
    #[test]
    fn test_lifetime() {
        let observable = Just::new(1);
        let subscription;
        {
            let a = 1;
            let on_next = |_| println!("{}", &a);

            let b = 1;
            let on_terminal = |_| println!("{}", &b);

            subscription = observable.subscribe_on(on_next, on_terminal);
        }
        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_fn() {
        struct MyStruct;
        impl MyStruct {
            fn test(self) {}
            fn mut_test(&mut self) {}
            // fn ref_test(&self) {}
        }
        let mut s1 = MyStruct;
        let s2 = MyStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe_on(
            |_| {
                s1.mut_test();
            },
            |_| {
                s2.test();
            },
        );
        drop(subscription); // keep the subscription alive
    }
}
