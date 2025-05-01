use super::just::Just;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Start<T>(Just<T>);

impl<T> Start<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T,
    {
        Self(Just::new(f()))
    }
}

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Start<T> {
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        self.0.subscribe(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::{Observable, observable_ext::ObservableExt},
        utils::tests_utils::checker::Checker,
    };

    #[test]
    fn test_completed() {
        let value = 111;
        let observable = Start::new(|| value + 222);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let observable = Start::new(|| &value);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let observable = Start::new(|| &mut value);
        let (checker, observer) = Checker::new();

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
        let value = 111;
        let observable = Start::new(|| value + 222);
        let (checker, observer) = Checker::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let value = 111;
        let observable = Start::new(|| value + 222);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[333]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[333]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let value = 111;
        let observable = Start::new(|| value + 222);
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let value = 111;
        let observable = Start::new(|| value + 222);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let value = 111;
        let observable = Start::new(|| value + 222);

        let _ = observable.buffer_with_count(1);
    }
}
