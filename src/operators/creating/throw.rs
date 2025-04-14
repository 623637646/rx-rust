use crate::{
    observable::{Observable, observable_ext::ObservableExt},
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
    pub fn new(error: E) -> Throw<E> {
        Throw(error)
    }
}

impl<'a, E, OR> Observable<'a, Infallible, E, OR> for Throw<E>
where
    OR: Observer<Infallible, E>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        observer.on_terminal(Terminal::Error(self.0));
        Subscription::new_none_disposal()
    }
}

impl<E> ObservableExt for Throw<E> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests_utils::checking_observer::CheckingObserver;

    #[test]
    fn test_error() {
        let observable = Throw::new(111);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(111));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let error = 111;

        let observable = Throw::new(&error);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut error = 111;

        let observable = Throw::new(&mut error);
        let checker: CheckingObserver<i32, i32> = CheckingObserver::new();

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
        let checker = CheckingObserver::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
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
}
