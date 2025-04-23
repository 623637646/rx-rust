use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'sub, T, OR> Observable<'sub, T, Infallible, OR> for Never
where
    OR: Observer<T, Infallible>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        Subscription::new_none_disposal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_unterminated() {
        let observable = Never;
        let checker: CheckingObserver<i32, Infallible> = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Never;
        let checker: CheckingObserver<i32, Infallible> = CheckingObserver::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Never;
        let checker_1: CheckingObserver<i32, Infallible> = CheckingObserver::new();
        let checker_2: CheckingObserver<i32, Infallible> = CheckingObserver::new();

        // Custom operations
        let observable_1 = observable.clone();
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

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
        let checker: CheckingObserver<Vec<i32>, Infallible> = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Never;

        let _ = observable.buffer_with_count(1);
    }
}
