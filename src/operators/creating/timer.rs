use super::just::Just;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    operators::utility::delay::Delay,
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timer<T, S>(Delay<Just<T>, S>);

impl<T, S> Timer<T, S> {
    pub fn new(value: T, delay: Duration, scheduler: S) -> Self
    where
        T: Send + 'static,
        S: Scheduler,
    {
        Self(Just::new(value).delay(delay, scheduler))
    }
}

impl<'a, T, OR, S> Observable<'a, T, Infallible, OR> for Timer<T, S>
where
    T: Send + 'static,
    OR: Observer<T, Infallible> + Send + 'static,
    S: Scheduler,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        self.0.subscribe(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::Observable, scheduler::tokio_scheduler::TokioScheduler,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[tokio::test]
    async fn test_completed() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();
        let checker_3 = CheckingObserver::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        let subscription_3 = observable_3.subscribe(checker_3.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subscription_1.unsubscribe();

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subscription_2.unsubscribe();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_completed());

        _ = subscription_3; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
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
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
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
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[tokio::test]
    async fn test_type_inference_without_subscribe() {
        // Custom operations
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);

        let _ = observable.buffer_with_count(1);
    }
}
