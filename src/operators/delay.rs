use crate::{
    observable::Observable,
    observer::{anonymous_observer::AnonymousObserver, event::Event, Observer},
    scheduler::Scheduler,
    subscription::Subscription,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// This is an observable that delays the next value and completed events from the source observable by a duration. The error and unsubscribed events will post immediately.
pub struct Delay<O, S> {
    source: O,
    delay: Duration,
    scheduler: Arc<S>,
}

impl<O, S> Delay<O, S> {
    pub fn new(source: O, delay: Duration, scheduler: S) -> Delay<O, S> {
        Delay {
            source,
            delay,
            scheduler: Arc::new(scheduler),
        }
    }
}

impl<O, S> Clone for Delay<O, S>
where
    O: Clone,
{
    fn clone(&self) -> Self {
        Delay {
            source: self.source.clone(),
            delay: self.delay,
            scheduler: self.scheduler.clone(),
        }
    }
}

impl<T, E, O, S> Observable<T, E> for Delay<O, S>
where
    O: Observable<T, E>,
    S: Scheduler,
    T: Send + 'static,
    E: Send + 'static,
{
    fn subscribe(self, observer: impl Observer<T, E>) -> Subscription {
        let scheduler = self.scheduler.clone();
        let delay = self.delay;
        let observer = Arc::new(observer);
        let disposals = Arc::new(Mutex::new(Vec::new()));
        let disposals_cloned = disposals.clone();
        let observer = AnonymousObserver::new(move |event: Event<T, E>| {
            let should_be_delay = match &event {
                Event::Next(_) => true,
                Event::Terminated(terminated) => match terminated {
                    crate::observer::event::Terminated::Completed => true,
                    crate::observer::event::Terminated::Error(_) => false,
                    crate::observer::event::Terminated::Unsubscribed => false,
                },
            };
            if should_be_delay {
                let observer = observer.clone();
                let disposal =
                    scheduler.schedule(move || observer.notify_if_unterminated(event), Some(delay));
                let disposal = disposal.to_boxed();
                disposals.lock().unwrap().push(disposal);
            } else {
                observer.notify_if_unterminated(event);
            }
        });
        let subscription = self.source.subscribe(observer);
        subscription.insert_disposal_action(move || {
            for disposal in disposals_cloned.lock().unwrap().drain(..) {
                disposal.dispose();
            }
        })
    }
}

/// Make the `Observable` delayable.
pub trait DelayableObservable<T, E> {
    /**
    Delay the events from the source observable by a duration.

    # Example
    ```rust
    use rx_rust::operators::just::Just;
    use rx_rust::operators::delay::DelayableObservable;
    use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use rx_rust::scheduler::tokio_scheduler::TokioScheduler;
    use std::time::Duration;
    #[tokio::main]
    async fn main() {
        let observable = Just::new(333);
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        observable.subscribe_on_event(|event| {
            println!("{:?}", event);
        });
    }
    ```
     */
    fn delay<S>(self, delay: Duration, scheduler: S) -> impl Observable<T, E>
    where
        S: Scheduler,
        T: Send + 'static,
        E: Send + 'static;
}

impl<O, T, E> DelayableObservable<T, E> for O
where
    O: Observable<T, E>,
{
    fn delay<S>(self, delay: Duration, scheduler: S) -> impl Observable<T, E>
    where
        S: Scheduler,
        T: Send + 'static,
        E: Send + 'static,
    {
        Delay::new(self, delay, scheduler)
    }
}

#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::event::Terminated, operators::create::Create,
        scheduler::tokio_scheduler::TokioScheduler, utils::checking_observer::CheckingObserver,
    };
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_completed() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Error(
                    "error".to_string(),
                )));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_error("error".to_owned()));
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_error("error".to_owned()));
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribed() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            subscription.unsubscribe()
        });
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unsubscribed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unsubscribed());
    }

    #[tokio::test]
    async fn test_unterminated() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_subscribe() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);

        let checker1 = CheckingObserver::new();
        let subscription1 = observable.clone().subscribe(checker1.clone());
        let checker2 = CheckingObserver::new();
        let subscription2 = observable.clone().subscribe(checker2.clone());

        assert!(checker1.is_values_matched(&[]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker1.is_values_matched(&[]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_completed());
        _ = subscription1; // keep the subscription alive
        _ = subscription2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_operate() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }
}
