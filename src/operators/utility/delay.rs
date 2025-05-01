use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscription::{Subscription, disposable::CallbackDisposal},
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// An observable that delays the next value and completed events from the source observable by a duration.
/// The error will be emitted immediately.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
    /// Creates a new `Delay` observable.
    ///
    /// # Arguments
    ///
    /// * `source` - The source observable to delay.
    /// * `delay` - The duration to delay each emission.
    /// * `scheduler` - The scheduler to use for timing the delay.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Delay<OE, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'static) -> Subscription<'sub> {
        let source_observer = Arc::new(Mutex::new(Some(observer)));
        let delay_observer = DelayObserver {
            source_observer: source_observer.clone(),
            delay: self.delay,
            scheduler: self.scheduler,
        };
        let disposal = CallbackDisposal::new(move || {
            source_observer.lock().unwrap().take();
        });
        let subscription = self.source.subscribe(delay_observer);
        subscription + disposal
    }
}

pub struct DelayObserver<OR, S> {
    source_observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: S,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<OR, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let observer = self.source_observer.clone();
        self.scheduler.schedule(
            move || {
                let mut observer = observer.lock().unwrap();
                if let Some(observer) = &mut *observer {
                    observer.on_next(value)
                }
            },
            Some(self.delay),
        );
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match &terminal {
            Terminal::Completed => {
                self.scheduler.schedule(
                    move || {
                        let observer = self.source_observer.lock().unwrap().take();
                        if let Some(observer) = observer {
                            observer.on_terminal(terminal);
                        }
                    },
                    Some(self.delay),
                );
            }
            Terminal::Error(_) => {
                let observer = self.source_observer.lock().unwrap().take();
                if let Some(observer) = observer {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}

#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        operators::creating::{create::Create, just::Just},
        scheduler::tokio_scheduler::TokioScheduler,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checker::Checker, test_struct::TestStruct},
    };

    #[tokio::test]
    async fn test_completed() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333, 444]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        let subscription_3 = observable_3.subscribe(observer_3);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        subscription_2.unsubscribe();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222, 333]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222, 333]));
        assert!(checker_3.is_error("error"));

        _ = subscription_3; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
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
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .delay(Duration::from_millis(50), TokioScheduler)
            .delay(Duration::from_millis(50), TokioScheduler);

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Delay::new(observable, Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_lifetime() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

            let (checker, observer) = Checker::new();
            subscription = observable.subscribe(observer);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Just::new(111);
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.delay(Duration::from_millis(100), TokioScheduler);

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.delay(Duration::from_millis(100), TokioScheduler);

        let _ = observable.buffer_with_count(1);
    }
}
