use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Never {
    fn subscribe(self, _: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        Subscription::new_none_disposal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::observable_ext::ObservableExt, utils::tests_utils::checker::Checker};

    #[test]
    fn test_unterminated() {
        let observable = Never;
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unsubscribed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Never;
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unsubscribed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unsubscribed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Never;
        let (checker_1, observer_1) = Checker::<i32, Infallible>::new();
        let (checker_2, observer_2) = Checker::<i32, Infallible>::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unsubscribed());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unsubscribed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Never;
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let observable = Never;

        let observable = observable.buffer_with_count(1);
        let (_, observer) = Checker::<Vec<i32>, Infallible>::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Never;

        let _ = observable.buffer_with_count(1);
    }
}
