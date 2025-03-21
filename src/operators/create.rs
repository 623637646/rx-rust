use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
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
/// The `handler` function is called when an observer subscribes to the observable. It receives
/// a `CreateObserver` which it can use to emit values and terminal events. The function should return a
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
///     move |value| println!("value: {}", value),
///     move |terminal: Terminal<String>| println!("terminal: {:?}", terminal),
/// );
/// ```
#[derive(Clone)]
pub struct Create<F> {
    handler: F,
}

impl<F> Create<F> {
    /// Creates a new `Create` observable.
    ///
    /// # Arguments
    ///
    /// * `handler` - The subscription handler function. It receives a `CreateObserver` which it can use to emit values and terminal events. The function should return a `Subscription` which can be used to manage the subscription.
    ///
    /// # Type Parameters
    ///
    /// * `OR` - The type of the observer that will receive events from the observable.
    /// * `D` - The type of the disposable that will be used to manage the subscription.
    pub fn new<OR>(handler: F) -> Create<F>
    where
        // Using `CreateObserver<OR>` instead of `OR` to make Create more easy to use. See more in `CreateObserver` "Why necessary".
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_wrap_observable`.
        F: FnMut(CreateObserver<OR>) -> Subscription,
    {
        Create { handler }
    }
}

impl<T, E, OR, F> Observable<T, E, OR> for Create<F>
where
    OR: Observer<T, E>,
    F: FnMut(CreateObserver<OR>) -> Subscription,
{
    fn subscribe(mut self, observer: OR) -> Subscription {
        (self.handler)(CreateObserver(observer))
    }
}

/// `CreateObserver` is a wrapper around an observer that is used by the `Create` operator.
/// It allows the `Create` operator to send values and terminal events to the wrapped observer.
///
/// # Type Parameters
/// - `OR`: The type of the wrapped observer.
///
/// # Why necessary
///
/// Using `CreateObserver<OR>` instead of directly using `OR` to make `Create` more easy to use.
/// Otherwise, this code can't be compiled:
/// ```rust
/// use rx_rust::operators::create::Create;
/// use rx_rust::observer::Terminal;
/// use rx_rust::subscription::Subscription;
/// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
/// use rx_rust::observer::Observer;
/// let observable = Create::new(|mut observer| { // The compiler can't infer the type of `OR` without using `CreateObserver<OR>`
///     observer.on_next(1);
///     observer.on_terminal(Terminal::<String>::Completed);
///     Subscription::new_none_disposal()
/// });
/// observable.subscribe_on(
///     move |value| {},
///     move |terminal| {},
/// );
/// ```
pub struct CreateObserver<OR>(OR);

impl<T, E, OR> Observer<T, E> for CreateObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.0.on_terminal(terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::Terminal, operators::just::Just, utils::checking_observer::CheckingObserver,
    };
    use std::time::Duration;
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
            let handle = tokio::spawn(async move {
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

    // TODO: Think about if Observable not confirm to Clone
    // #[tokio::test]
    // async fn test_async() {
    //     let (tx, rx) = tokio::sync::oneshot::channel();
    //     let observable = Create::new(|mut observer| {
    //         observer.on_next(333);
    //         let handle = tokio::spawn(async move {
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
}
