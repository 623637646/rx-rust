use super::just::Just;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timer<T, S> {
    value: T,
    delay: Duration,
    scheduler: S,
}

impl<T, S> Timer<T, S> {
    pub fn new(value: T, delay: Duration, scheduler: S) -> Self {
        Self {
            value,
            delay,
            scheduler,
        }
    }
}

impl<'sub, T, S> Observable<'static, 'sub, T, Infallible> for Timer<T, S>
where
    T: Send + 'static,
    S: Scheduler + Send,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + Send + 'static,
    ) -> Subscription<'sub> {
        Just::new(self.value)
            .delay(self.delay, self.scheduler)
            .subscribe(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::Observable, scheduler::tokio_scheduler::TokioScheduler,
        utils::tests_utils::checker::Checker,
    };

    #[tokio::test]
    async fn test_completed() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_active());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_active());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        let subscription_3 = observable_3.subscribe(observer_3);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_active());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_active());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_active());

        subscription_1.unsubscribe();

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_dropped());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_active());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_active());

        subscription_2.unsubscribe();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_dropped());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_dropped());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_completed());

        _ = subscription_3; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let (checker, observer) = Checker::new();

        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_active());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_active());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_active());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_active());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_active());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_active());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_clone() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let _ = observable.clone();
    }

    #[tokio::test]
    async fn test_type_inference_with_subscribe() {
        // Custom operations
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);

        let observable = observable.buffer_with_count(1);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[tokio::test]
    async fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);

        let _ = observable.buffer_with_count(1);
    }
}
