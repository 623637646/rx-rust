use super::from_iter::FromIter;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Repeat<T> {
    value: T,
    n: usize,
}

impl<T> Repeat<T> {
    pub fn new(value: T, n: usize) -> Self {
        Self { value, n }
    }
}

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Repeat<T>
where
    T: Clone,
{
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        FromIter::new(std::iter::repeat_n(self.value, self.n)).subscribe(observer)
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
        let observable = Repeat::new(3, 4);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 1;
        let observable = Repeat::new(&value, 4);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value, &value, &value, &value]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Repeat::new(3, 4);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker.is_completed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let observable = Repeat::new(3, 4);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Repeat::new(3, 4);
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let observable = Repeat::new(3, 4);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Repeat::new(3, 4);

        let _ = observable.buffer_with_count(1);
    }
}
