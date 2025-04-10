use crate::{
    observable::Observable, observer::Observer, scheduler::Scheduler, subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};

#[derive(Clone)]
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

impl<'a, OR, S> Observable<'a, usize, Infallible, OR> for Interval<S>
where
    OR: Observer<usize, Infallible> + Send + 'static,
    S: Scheduler,
{
    fn subscribe(self, mut observer: OR) -> Subscription<'a> {
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
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        scheduler::tokio_scheduler::TokioScheduler,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[tokio::test]
    async fn test_no_delay() {
        let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);
        let checker = CheckingObserver::new();

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
    async fn test_with_delay() {
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let checker = CheckingObserver::new();

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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

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

        drop(subscription_2); // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let checker = CheckingObserver::new();

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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);

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
}
