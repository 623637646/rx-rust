use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    utils::disposal::Disposal,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// This is an observable that delays the next value and completed events from the source observable by a duration. The error and unsubscribed events will post immediately.
pub struct Delay<U, S> {
    upstream_observable: U,
    delay: Duration,
    scheduler: Arc<S>,
}

impl<U, S> Delay<U, S> {
    pub fn new(upstream_observable: U, delay: Duration, scheduler: S) -> Delay<U, S> {
        Delay {
            upstream_observable,
            delay,
            scheduler: Arc::new(scheduler),
        }
    }
}

impl<U, S> Clone for Delay<U, S>
where
    U: Clone,
{
    fn clone(&self) -> Self {
        Delay {
            upstream_observable: self.upstream_observable.clone(),
            delay: self.delay,
            scheduler: self.scheduler.clone(),
        }
    }
}

impl<T, E, O, U, S> Observable<T, E, O> for Delay<U, S>
where
    O: Observer<T, E>,
    U: Observable<T, E, DelayObserver<O, S>>,
    S: Scheduler,
    T: Send + 'static,
    E: Send + 'static,
{
    fn subscribe(self, observer: O) -> Disposal {
        let observer = DelayObserver::new(observer, self.scheduler, self.delay);
        self.upstream_observable.subscribe(observer)
    }
}

struct DelayObserver<U, S> {
    upstream_observer: Arc<Mutex<Option<U>>>,
    scheduler: Arc<S>,
    delay: Duration,
    disposals: Arc<Mutex<Vec<Disposal>>>,
}

impl<U, S> DelayObserver<U, S> {
    fn new(upstream_observer: U, scheduler: Arc<S>, delay: Duration) -> DelayObserver<U, S> {
        DelayObserver {
            upstream_observer: Arc::new(Mutex::new(Some(upstream_observer))),
            scheduler,
            delay,
            disposals: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl<T, E, U, S> Observer<T, E> for DelayObserver<U, S>
where
    U: Observer<T, E>,
    S: Scheduler,
    T: Send + 'static,
    E: Send + 'static,
{
    fn on_next(&mut self, value: T) {
        let observer = self.upstream_observer.clone();
        let disposals = self.disposals.clone();
        let disposal = self.scheduler.schedule(
            move || {
                let mut observer = observer.lock().unwrap();
                if let Some(observer) = observer.as_mut() {
                    observer.on_next(value)
                }
                // TODO: remove disposal
            },
            Some(self.delay),
        );
        disposals.lock().unwrap().push(disposal);
    }

    fn on_terminal(self: Box<Self>, terminated: Terminal<E>) {
        match &terminated {
            Terminal::Error(_) => {
                let upstream_observer = self.upstream_observer.lock().unwrap().take().unwrap();
                Box::new(upstream_observer).on_terminal(terminated);
            }
            Terminal::Completed => {
                let disposals = self.disposals.clone();
                let delay = self.delay;
                let disposal = self.scheduler.clone().schedule(
                    move || {
                        let upstream_observer =
                            self.upstream_observer.lock().unwrap().take().unwrap();
                        Box::new(upstream_observer).on_terminal(terminated);
                        drop(self.disposals); // TODO: why this is needed?
                    },
                    Some(delay),
                );
                disposals.lock().unwrap().push(disposal);
            }
        }
    }
}

/// Make the `Observable` delayable.
pub trait DelayableObservable<T, E, O, S>
where
    O: Observer<T, E>,
    S: Scheduler,
    T: Send + 'static,
    E: Send + 'static,
{
    fn delay(self, delay: Duration, scheduler: S) -> impl Observable<T, E, O>;
}

impl<ObservableType, T, E, O, S> DelayableObservable<T, E, O, S> for ObservableType
where
    ObservableType: Observable<T, E, DelayObserver<O, S>>,
    O: Observer<T, E>,
    S: Scheduler,
    T: Send + 'static,
    E: Send + 'static,
{
    fn delay(self, delay: Duration, scheduler: S) -> impl Observable<T, E, O> {
        Delay::new(self, delay, scheduler)
    }
}
#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operators::{create::Create, just::Just},
        scheduler::tokio_scheduler::TokioScheduler,
        utils::checking_observer::CheckingObserver,
    };
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test1() {
        let observable = Just::new(1);
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let disposal = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_completed());
        _ = disposal; // keep the disposal alive
    }

    #[tokio::test]
    async fn test_completed() {
        let observable = Create::new(|observer| {
            let observer = Arc::new(Mutex::new(Some(observer)));
            if let Some(observer) = observer.lock().unwrap().as_mut() {
                observer.on_next(1);
            }
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                if let Some(observer) = observer_cloned.lock().unwrap().as_mut() {
                    observer.on_next(2);
                }
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                if let Some(observer) = observer_cloned.lock().unwrap().take() {
                    observer.on_terminal(Terminal::<String>::Completed);
                }
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                if let Some(observer) = observer_cloned.lock().unwrap().as_mut() {
                    observer.on_next(3);
                }
            });
            Disposal::new_no_action()
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

    // #[tokio::test]
    // async fn test_error() {
    //     let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    //         let observer = Arc::new(observer);
    //         observer.notify_if_unterminated(Event::Next(1));
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(2));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Error(
    //                 "error".to_string(),
    //             )));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(3));
    //         });
    //         Subscription::new_non_disposal_action(observer)
    //     });
    //     let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
    //     let checker = CheckingObserver::new();
    //     let subscription = observable.subscribe(checker.clone());
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(5)).await;
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_error("error".to_owned()));
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_error("error".to_owned()));
    //     _ = subscription; // keep the subscription alive
    // }

    // #[tokio::test]
    // async fn test_unsubscribed() {
    //     let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    //         let observer = Arc::new(observer);
    //         observer.notify_if_unterminated(Event::Next(1));
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(2));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(3));
    //         });
    //         Subscription::new_non_disposal_action(observer)
    //     });
    //     let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
    //     let checker = CheckingObserver::new();
    //     let subscription = observable.subscribe(checker.clone());
    //     tokio::spawn(async move {
    //         tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //         subscription.unsubscribe()
    //     });
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(5)).await;
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unsubscribed());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unsubscribed());
    // }

    // #[tokio::test]
    // async fn test_unterminated() {
    //     let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    //         let observer = Arc::new(observer);
    //         observer.notify_if_unterminated(Event::Next(1));
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(2));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(3));
    //         });
    //         Subscription::new_non_disposal_action(observer)
    //     });
    //     let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
    //     let checker = CheckingObserver::new();
    //     let subscription = observable.subscribe(checker.clone());
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(5)).await;
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2, 3]));
    //     assert!(checker.is_unterminated());
    //     _ = subscription; // keep the subscription alive
    // }

    // #[tokio::test]
    // async fn test_multiple_subscribe() {
    //     let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    //         let observer = Arc::new(observer);
    //         observer.notify_if_unterminated(Event::Next(1));
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(2));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    //             observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(3));
    //         });
    //         Subscription::new_non_disposal_action(observer)
    //     });
    //     let observable = observable.delay(Duration::from_millis(10), TokioScheduler);

    //     let checker1 = CheckingObserver::new();
    //     let subscription1 = observable.clone().subscribe(checker1.clone());
    //     let checker2 = CheckingObserver::new();
    //     let subscription2 = observable.clone().subscribe(checker2.clone());

    //     assert!(checker1.is_values_matched(&[]));
    //     assert!(checker1.is_unterminated());
    //     assert!(checker2.is_values_matched(&[]));
    //     assert!(checker2.is_unterminated());
    //     sleep(Duration::from_millis(5)).await;
    //     assert!(checker1.is_values_matched(&[]));
    //     assert!(checker1.is_unterminated());
    //     assert!(checker2.is_values_matched(&[]));
    //     assert!(checker2.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker1.is_values_matched(&[1]));
    //     assert!(checker1.is_unterminated());
    //     assert!(checker2.is_values_matched(&[1]));
    //     assert!(checker2.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker1.is_values_matched(&[1, 2]));
    //     assert!(checker1.is_unterminated());
    //     assert!(checker2.is_values_matched(&[1, 2]));
    //     assert!(checker2.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker1.is_values_matched(&[1, 2]));
    //     assert!(checker1.is_completed());
    //     assert!(checker2.is_values_matched(&[1, 2]));
    //     assert!(checker2.is_completed());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker1.is_values_matched(&[1, 2]));
    //     assert!(checker1.is_completed());
    //     assert!(checker2.is_values_matched(&[1, 2]));
    //     assert!(checker2.is_completed());
    //     _ = subscription1; // keep the subscription alive
    //     _ = subscription2; // keep the subscription alive
    // }

    // #[tokio::test]
    // async fn test_multiple_operate() {
    //     let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    //         let observer = Arc::new(observer);
    //         observer.notify_if_unterminated(Event::Next(1));
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(2));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    //             observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
    //         });
    //         let observer_cloned = observer.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    //             observer_cloned.notify_if_unterminated(Event::Next(3));
    //         });
    //         Subscription::new_non_disposal_action(observer)
    //     });
    //     let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
    //     let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
    //     let checker = CheckingObserver::new();
    //     let subscription = observable.subscribe(checker.clone());
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(5)).await;
    //     assert!(checker.is_values_matched(&[]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_unterminated());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_completed());
    //     sleep(Duration::from_millis(10)).await;
    //     assert!(checker.is_values_matched(&[1, 2]));
    //     assert!(checker.is_completed());
    //     _ = subscription; // keep the subscription alive
    // }
}
