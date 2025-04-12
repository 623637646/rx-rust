use super::just::Just;
use crate::{
    operators::utility::delay::{Delay, DelayableObservable},
    scheduler::Scheduler,
};
use std::time::Duration;

pub fn timer<T, S>(value: T, delay: Duration, scheduler: S) -> Delay<Just<T>, S>
where
    T: Send + 'static,
    S: Scheduler,
{
    Just::new(value).delay(delay, scheduler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::Observable, scheduler::tokio_scheduler::TokioScheduler,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[tokio::test]
    async fn test_timer() {
        let observable = timer(111, Duration::from_millis(100), TokioScheduler);
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
}
