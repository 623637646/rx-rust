use super::from_iter::FromIter;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::{convert::Infallible, ops::RangeBounds};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Range<I>(I);

impl<I> Range<I> {
    pub fn new<T>(range: I) -> Self
    where
        I: IntoIterator<Item = T> + RangeBounds<T>,
    {
        Self(range)
    }
}

impl<'or, 'sub, T, I> Observable<'or, 'sub, T, Infallible> for Range<I>
where
    I: IntoIterator<Item = T>,
{
    fn subscribe(self, observer: impl Observer<T, Infallible> + Send + 'or) -> Subscription<'sub> {
        FromIter::new(self.0).subscribe(observer)
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
    fn test_completed_range() {
        let source = 100..103;
        let observable = Range::new(source);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_range_inclusive() {
        let source = 100..=103;
        let observable = Range::new(source);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[100, 101, 102, 103]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let source = 100..103;
        let observable = Range::new(source);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let source = 100..103;
        let observable = Range::new(source);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[100, 101, 102]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[100, 101, 102]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let source = 100..103;
        let observable = Range::new(source);
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let source = 100..103;
        let observable = Range::new(source);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let source = 100..103;
        let observable = Range::new(source);

        let _ = observable.buffer_with_count(1);
    }
}
