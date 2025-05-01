use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

/// This is an observable that emits an error.
///
/// # Example
/// ```rust
/// use rx_rust::operators::creating::throw::Throw;
/// use rx_rust::observable::observable_ext::ObservableExt;
/// use std::convert::Infallible;
/// use rx_rust::observer::Terminal;
/// let observable = Throw::new("My error");
/// observable.subscribe_with_callback(
///     |_| {},
///     |terminal| println!("Terminal event: {:?}", terminal)
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throw<E>(E);

impl<E> Throw<E> {
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<'or, 'sub, E> Observable<'or, 'sub, Infallible, E> for Throw<E> {
    fn subscribe(self, observer: impl Observer<Infallible, E> + Send + 'or) -> Subscription<'sub> {
        observer.on_terminal(Terminal::Error(self.0));
        Subscription::new_none_disposal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::observable_ext::ObservableExt, utils::tests_utils::checker::Checker};

    #[test]
    fn test_error() {
        let observable = Throw::new(111);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(111));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let error = 111;

        let observable = Throw::new(&error);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut error = 111;

        let observable = Throw::new(&mut error);
        let (checker, observer) = Checker::<i32, i32>::new();

        let checker_cloned = checker.clone();
        let subscription = observable.subscribe_with_callback(
            |_| unreachable!(),
            |terminal| match terminal {
                Terminal::Completed => unreachable!(),
                Terminal::Error(error) => {
                    checker_cloned.on_terminal(Terminal::Error(*error));
                    *error = 222;
                }
            },
        );

        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(111));
        assert_eq!(error, 222);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Throw::new(111);
        let (checker, observer) = Checker::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(111));

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(111));
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Throw::new(111);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_error(111));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_1.is_error(111));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Throw::new(111);
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let observable = Throw::new(111);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Throw::new(111);

        let _ = observable.buffer_with_count(1);
    }
}
