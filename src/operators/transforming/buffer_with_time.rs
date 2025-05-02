use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_pan: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTime<OE, S> {
    pub fn new(source: OE, time_pan: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_pan,
            scheduler,
            delay,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTime<OE, S>
where
    T: Send + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(self, observer: impl Observer<Vec<T>, E> + Send + 'static) -> Subscription<'sub> {
        let observer = BufferWithTimeObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            values: Arc::new(Mutex::new(Vec::default())),
        };
        let observer_cloned = observer.clone();
        let disposal = self.scheduler.schedule_period(
            move |_| {
                if let Some(observer) = observer_cloned.observer.lock().unwrap().as_mut() {
                    let mut values = observer_cloned.values.lock().unwrap();
                    observer.on_next(std::mem::take(&mut values));
                }
            },
            self.time_pan,
            self.delay,
        );
        self.source.subscribe(observer) + disposal
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferWithTimeObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.lock().unwrap().push(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        if let Some(mut observer) = self.observer.lock().unwrap().take() {
            match terminal {
                Terminal::Completed => {
                    let mut values = self.values.lock().unwrap();
                    if !values.is_empty() {
                        observer.on_next(std::mem::take(&mut values));
                    }
                    observer.on_terminal(Terminal::Completed);
                }
                Terminal::Error(error) => {
                    observer.on_terminal(Terminal::Error(error));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::{Observable, observable_ext::ObservableExt},
        observer::Terminal,
        operators::creating::create::Create,
        scheduler::tokio_scheduler::TokioScheduler,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checker::Checker, test_struct::TestStruct},
    };

    #[tokio::test]
    async fn test_completed_last_empty() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_last_not_empty() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_no_delay() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable =
            observable.buffer_with_time(Duration::from_millis(100), TokioScheduler, None);

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![], vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_small_delay() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(30)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![], vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_empty() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_not_empty() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
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
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unsubscribed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unsubscribed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unsubscribed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unsubscribed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(111);
        });
        handle.await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(222);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(333);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unsubscribed());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unsubscribed());
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
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
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_completed());

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
            .buffer_with_time(
                Duration::from_millis(90),
                TokioScheduler,
                Some(Duration::from_millis(90)),
            )
            .buffer_with_time(
                Duration::from_millis(100),
                TokioScheduler,
                Some(Duration::from_millis(100)),
            );

        let subscription = observable.clone().subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![vec![]]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![vec![]]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]], vec![vec![222, 333]]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = BufferWithTime::new(
            observable,
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

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
                observer.on_next(111);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = observable.buffer_with_time(
                Duration::from_millis(100),
                TokioScheduler,
                Some(Duration::from_millis(100)),
            );

            let (_, observer) = Checker::<_, ()>::new();
            subscription = observable.subscribe(observer);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
        let observable = observable.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );
        let _ = observable.clone(); // make sure it's Clone when T is not Clone.
    }

    #[tokio::test]
    async fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let observable = observable.buffer_with_count(1);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time(
            Duration::from_millis(100),
            TokioScheduler,
            Some(Duration::from_millis(100)),
        );

        let _ = observable.buffer_with_count(1);
    }
}
