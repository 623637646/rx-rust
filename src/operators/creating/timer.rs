use super::just::Just;
use crate::{
    observable::Observable,
    observer::Observer,
    operators::utility::delay::{Delay, DelayableObservable},
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
    async fn test_timer() {
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

    #[test]
    fn test_clone() {
        let observable = Timer::new(111, Duration::from_millis(100), TokioScheduler);
        let _ = observable.clone();
    }
}
