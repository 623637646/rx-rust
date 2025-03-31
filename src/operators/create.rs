use crate::{
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use std::marker::PhantomData;

/// The `Create` struct is an implementation of the `Observable` trait that allows creating an observable
/// from a custom subscription function. The subscription function is provided by the user and is responsible
/// for emitting values and terminal events to the observer.
///
/// # Type Parameters
///
/// * `F` - The type of the subscription function.
///
/// The `handler` function is called when an observer subscribes to the observable. It receives
/// a `BoxedObserver` which it can use to emit values and terminal events. The function should return a
/// `Subscription` which can be used to manage the subscription.
///
/// # Example
/// ```rust
/// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
/// use rx_rust::observer::Observer;
/// use rx_rust::subscription::Subscription;
/// use rx_rust::operators::create::Create;
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
pub struct Create<'a, 'b, F> {
    handler: F,
    _marker: PhantomData<(&'a (), &'b ())>,
}

impl<'a, 'b, F> Create<'a, 'b, F> {
    /// Creates a new `Create` observable.
    ///
    /// # Arguments
    ///
    /// * `handler` - The subscription handler function. It receives a `BoxedObserver` which it can use to emit values and terminal events. The function should return a `Subscription` which can be used to manage the subscription.
    pub fn new<T, E>(handler: F) -> Create<'a, 'b, F>
    where
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_wrap_observable`.
        F: FnOnce(BoxedObserver<'b, T, E>) -> Subscription<'a>,
    {
        Create {
            handler,
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, T, E, OR, F> Observable<'a, T, E, OR> for Create<'a, 'b, F>
where
    OR: Observer<T, E> + Send + 'b,
    F: FnOnce(BoxedObserver<'b, T, E>) -> Subscription<'a>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        (self.handler)(BoxedObserver::new(observer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt, observer::Terminal,
        operators::just::Just, utils::checking_observer::CheckingObserver,
    };
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tokio::time::sleep;

    #[test]
    fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
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
            Subscription::new_none_disposal()
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
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        subscription.unsubscribe(); // unsubscribe
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
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

    #[test]
    fn test_multiple_subscribe_with_different_type() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });
        let checker = CheckingObserver::new();
        observable.clone().subscribe(checker.clone());
        observable.subscribe_on(
            |value| println!("value: {}", value),
            |terminal| println!("terminal: {:?}", terminal),
        );
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    // TODO: Think about if Observable not confirm to Clone
    // #[tokio::test]
    // async fn test_async() {
    //     let (tx, rx) = tokio::sync::oneshot::channel();
    //     let observable = Create::new(|mut observer| {
    //         observer.on_next(333);
    //         let handle = tokio::spawn(async {
    //             tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    //             observer.on_next(444);
    //             observer.on_terminal(Terminal::<String>::Completed);
    //             tx.send(()).unwrap();
    //         });
    //       Subscription::new_with_disposal_callback(move || handle.abort())
    //     });
    //     let checker = CheckingObserver::new();
    //     observable.subscribe(checker.clone());
    //     assert!(checker.is_values_matched(&[333]));
    //     assert!(checker.is_unterminated());
    //     tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    //     assert!(checker.is_values_matched(&[333, 444]));
    //     assert!(checker.is_completed());
    // }

    #[test]
    fn test_wrap_observable() {
        let observable = Create::new(|observer| Just::new(333).subscribe(observer));

        let checker = CheckingObserver::new();
        observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[tokio::test]
    async fn test_boxed_observer_in_arc_mutex() {
        let observable = Create::new(|observer| {
            let observer = Arc::new(Mutex::new(observer));
            let handle = tokio::spawn(async {
                let mut observer = Arc::try_unwrap(observer)
                    .unwrap_or_else(|_| panic!())
                    .into_inner()
                    .unwrap();
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    // Test with lifetime. See more for the git commit.
    #[test]
    fn test_lifetime() {
        let observable = Create::new(|mut observer| {
            observer.on_next(&1);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });

        let subscription;
        {
            let b = 1;
            let checker = CheckingObserver::new();
            checker.is_values_matched(&[&b]);
            subscription = observable.subscribe(checker);
        }
        _ = subscription; // keep the subscription alive
    }
}
