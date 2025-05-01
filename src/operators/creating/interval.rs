use crate::{
    observable::Observable, observer::Observer, scheduler::Scheduler, subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Interval<S> {
    period: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<S> Interval<S> {
    pub fn new(period: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            period,
            scheduler,
            delay,
        }
    }
}

impl<'sub, S> Observable<'static, 'sub, usize, Infallible> for Interval<S>
where
    S: Scheduler,
{
    fn subscribe(
        self,
        mut observer: impl Observer<usize, Infallible> + Send + 'static,
    ) -> Subscription<'sub> {
        let disposal = self.scheduler.schedule_period(
            move |count| {
                observer.on_next(count);
            },
            self.period,
            self.delay,
        );
        Subscription::new_with_disposal(disposal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt, scheduler::tokio_scheduler::TokioScheduler,
        utils::tests_utils::checker::Checker,
    };

    #[tokio::test]
    async fn test_completed_no_delay() {
        let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(checker.clone());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        subscription.unsubscribe();

        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_completed_with_delay() {
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        subscription.unsubscribe();

        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();

        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2, 3]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2, 3, 4]));
        assert!(checker_2.is_unterminated());

        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();

        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();
        subscription_2.unsubscribe();

        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[0, 1, 2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[0, 1, 2]));
        assert!(checker_2.is_unterminated());
    }

    #[tokio::test]
    async fn test_clone() {
        let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);
        let _ = observable.clone();
    }

    #[tokio::test]
    async fn test_type_inference_with_subscribe() {
        // Custom operations
        let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(checker);
    }

    #[tokio::test]
    async fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);

        let _ = observable.buffer_with_count(1);
    }
}
