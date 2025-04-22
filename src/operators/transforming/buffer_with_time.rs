use super::buffer::{Buffer, BufferObserver};
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    operators::{
        creating::interval::Interval,
        others::{
            map_infallible_to_error::MapInfallibleToError, map_value_to_void::MapValueToVoid,
        },
    },
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::time::Duration;

type BufferWithTimeType<OE, S> =
    Buffer<OE, MapInfallibleToError<MapValueToVoid<usize, Interval<S>>>>;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S>(BufferWithTimeType<OE, S>);

impl<OE, S> BufferWithTime<OE, S> {
    pub fn new(source: OE, time_pan: Duration, scheduler: S) -> Self {
        let boundary = Interval::new(time_pan, scheduler, Some(time_pan))
            .map_value_to_void()
            .map_infallible_to_error();
        let buffer = source.buffer(boundary);
        Self(buffer)
    }
}

impl<'sub, T, E, OR, OE, S> Observable<'sub, Vec<T>, E, OR> for BufferWithTime<OE, S>
where
    T: Send + 'static,
    E: 'static,
    OR: Observer<Vec<T>, E> + Send + 'static,
    OE: Observable<'sub, T, E, BufferObserver<T, OR>>,
    S: Scheduler,
{
    fn subscribe(self, observer: OR) -> Subscription<'sub> {
        self.0.subscribe(observer)
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
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };

    #[tokio::test]
    async fn test_completed_last_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
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
    async fn test_error_last_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);
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
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
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
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);
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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .buffer_with_time(Duration::from_millis(90), TokioScheduler)
            .buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.clone().subscribe(checker.clone());
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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable =
            BufferWithTime::new(observable, Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
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

            let observable =
                observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);

            let checker: CheckingObserver<_, ()> = CheckingObserver::new();
            subscription = observable.subscribe(checker.clone());
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
        let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler);
        let _ = observable.clone(); // make sure it's Clone when T is not Clone.
    }

    #[tokio::test]
    async fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time(Duration::from_millis(100), TokioScheduler);

        let _ = observable.buffer_with_count(1);
    }
}
