mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Terminal},
    operators::{creating::create::Create, transforming::buffer_with_time::BufferWithTime},
    scheduler::tokio_scheduler::TokioScheduler,
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_struct::TestStruct};

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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_completed());
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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_completed_no_delay() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_time(Duration::from_millis(100), TokioScheduler, None);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![], vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_completed());
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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![], vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![], vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_completed());
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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_error("error"));
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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_error("error"));
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
    let _subscription_2 = observable_2.subscribe(observer_2);
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
    assert!(checker_1.is_values_matched(&[vec![]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![]]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[vec![]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![]]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    subject.on_next(333);
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker_2.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker_2.is_completed());
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
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(333);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_dropped());
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

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_terminal) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
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
    assert!(checker_1.is_values_matched(&[vec![]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![]]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[vec![]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![]]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    subject.on_next(333);
    assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
    assert!(checker_2.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker_1.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker_2.is_completed());
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

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![vec![]]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![vec![]]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]], vec![vec![222, 333]]]));
    assert!(checker.is_completed());
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

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![]]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![], vec![111]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
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
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_terminal(Terminal::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.buffer_with_time(
        Duration::from_millis(100),
        TokioScheduler,
        Some(Duration::from_millis(100)),
    );
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
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

    observable.buffer_with_count(1);
}
