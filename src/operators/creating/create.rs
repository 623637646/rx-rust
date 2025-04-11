use crate::{
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
    subscription::Subscription,
};

/// The `Create` struct is an implementation of the `Observable` trait that allows creating an observable
/// from a custom subscription function. The subscription function is provided by the user and is responsible
/// for emitting values and terminal events to the observer.
///
/// # Type Parameters
///
/// * `F` - The type of the subscription function.
///
/// The `builder` function is called when an observer subscribes to the observable. It receives
/// a `BoxedObserver` which it can use to emit values and terminal events. The function should return a
/// `Subscription` which can be used to manage the subscription.
///
/// # Example
/// ```rust
/// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
/// use rx_rust::observer::Observer;
/// use rx_rust::subscription::Subscription;
/// use rx_rust::operators::creating::create::Create;
/// use rx_rust::observer::Terminal;
/// let observable = Create::new(|mut observer| {
///     observer.on_next(1);
///     observer.on_next(2);
///     observer.on_next(3);
///     observer.on_terminal(Terminal::Completed);
///     Subscription::new_none_disposal()
/// });
/// observable.subscribe_on(
///     |value| println!("value: {}", value),
///     |terminal: Terminal<String>| println!("terminal: {:?}", terminal),
/// );
/// ```
#[derive(Clone)]
pub struct Create<F>(F);

impl<F> Create<F> {
    /// Creates a new `Create` observable.
    ///
    /// # Arguments
    ///
    /// * `builder` - The subscription builder function. It receives a `BoxedObserver` which it can use to emit values and terminal events. The function should return a `Subscription` which can be used to manage the subscription.
    pub fn new<'a, 'b, T, E>(builder: F) -> Create<F>
    where
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_wrap_observable`.
        F: FnOnce(BoxedObserver<'b, T, E>) -> Subscription<'a>,
    {
        Create(builder)
    }
}

impl<'a, 'b, T, E, OR, F> Observable<'a, T, E, OR> for Create<F>
where
    OR: Observer<T, E> + Send + 'b,
    F: FnOnce(BoxedObserver<'b, T, E>) -> Subscription<'a>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        self.0(BoxedObserver::new(observer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        observer::Terminal,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };
    use std::time::Duration;

    #[test]
    fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            Subscription::new_none_disposal()
        });

        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                observer.on_next(2);
                tokio::time::sleep(Duration::from_millis(100)).await;
                observer.on_next(3);
                tokio::time::sleep(Duration::from_millis(100)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[1]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[1]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[1, 2]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe(); // unsubscribe

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[1, 2, 3]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[1, 2, 3]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let observable = Create::new(|mut observer| {
            observer.on_next(&value);
            observer.on_terminal(Terminal::Error(&error));
            Subscription::new_none_disposal()
        });
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let mut error = 222;

        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value);
            observer.on_terminal(Terminal::Error(&mut error));
            Subscription::new_none_disposal()
        });
        let checker = CheckingObserver::new();

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_on(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| match terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    checker_cloned_2.on_terminal(Terminal::Error(*error));
                    *error *= 2;
                }
            },
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error(222));
        assert_eq!(value, 222);
        assert_eq!(error, 444);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                observer.on_next(2);
                tokio::time::sleep(Duration::from_millis(100)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let checker = CheckingObserver::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_a() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let checker = CheckingObserver::new();
            checker.is_values_matched(&[1]);
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_b() {
        let life_marker = TestStruct;
        let observable = Create::new(|mut observer| {
            observer.on_next(&life_marker);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });

        // OK

        // Error
        // drop(life_marker);

        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_wrap_observable() {
        let mut subject = PublishSubject::default();
        let subject_cloned = subject.clone();
        let observable = Create::new(|observer| subject_cloned.subscribe(observer));
        let checker = CheckingObserver::new();

        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subscription.unsubscribe();

        subject.on_next(222);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_fn() {
        let s = TestStruct;

        let _ = Create::new(|mut observer| {
            s.consume();
            observer.on_next(111);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
    }
}
