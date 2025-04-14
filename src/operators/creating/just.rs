use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

/// This is an observable that emits a single value then completes.
///
/// # Example
/// ```rust
/// use rx_rust::operators::creating::just::Just;
/// use rx_rust::observable::observable_ext::ObservableExt;
/// use std::convert::Infallible;
/// use rx_rust::observer::Terminal;
/// let observable = Just::new(123);
/// observable.subscribe_with_callback(
///     |value| println!("Next value: {}", value),
///     |terminal| println!("Terminal event: {:?}", terminal)
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Just<T>(T);

impl<T> Just<T> {
    /// Creates a new `Just` observable with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to emit.
    pub fn new(value: T) -> Just<T> {
        Just(value)
    }
}

impl<'a, T, OR> Observable<'a, T, Infallible, OR> for Just<T>
where
    OR: Observer<T, Infallible>,
{
    fn subscribe(self, mut observer: OR) -> Subscription<'a> {
        observer.on_next(self.0);
        observer.on_terminal(Terminal::Completed);
        Subscription::new_none_disposal()
    }
}

impl<T> ObservableExt for Just<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests_utils::checking_observer::CheckingObserver;

    #[test]
    fn test_completed() {
        let observable = Just::new(111);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;

        let observable = Just::new(&value);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;

        let observable = Just::new(&mut value);
        let checker = CheckingObserver::new();

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_with_callback(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| checker_cloned_2.on_terminal(terminal),
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        assert_eq!(value, 222);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Just::new(111);
        let checker = CheckingObserver::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Just::new(111);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Just::new(111);
        let _ = observable.clone();
    }
}
