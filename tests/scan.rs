mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::scan::Scan},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(0, |last, value| last + value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(1);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    subject.on_next(2);
    assert_eq!(checker.values(), [1, 3]);
    assert!(checker.is_active());

    subject.on_next(3);
    assert_eq!(checker.values(), [1, 3, 6]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1, 3, 6]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(1, |last, value| last * value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(1);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    subject.on_next(2);
    assert_eq!(checker.values(), [1, 2]);
    assert!(checker.is_active());

    subject.on_next(3);
    assert_eq!(checker.values(), [1, 2, 6]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [1, 2, 6]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(0, |last, value| last + value);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(1);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());

    subject.on_next(2);
    assert_eq!(checker_1.values(), [1, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 3]);
    assert!(checker_2.is_active());

    subject.on_next(3);
    assert_eq!(checker_1.values(), [1, 3, 6]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 3, 6]);
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 3, 6]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 3, 6]);
    assert!(checker_2.is_active());

    subject.on_next(4);
    assert_eq!(checker_1.values(), [1, 3, 6]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 3, 6, 10]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 3, 6]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 3, 6, 10]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_ref() {
    let value_0 = 0;
    let value_1 = 1;
    let value_2 = 2;
    let value_3 = 3;
    let value_4 = 4;
    let value_5 = 5;
    let error = -1;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan((&value_0, 0), |last, value| (value, last.1 + *value));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(&value_1);
    assert_eq!(checker.values(), [(&value_1, 1)]);
    assert!(checker.is_active());

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [(&value_1, 1), (&value_2, 3)]);
    assert!(checker.is_active());

    subject.on_next(&value_3);
    assert_eq!(
        checker.values(),
        [(&value_1, 1), (&value_2, 3), (&value_3, 6)]
    );
    assert!(checker.is_active());

    subject.on_next(&value_4);
    assert_eq!(
        checker.values(),
        [(&value_1, 1), (&value_2, 3), (&value_3, 6), (&value_4, 10)]
    );
    assert!(checker.is_active());

    subject.on_next(&value_5);
    assert_eq!(
        checker.values(),
        [
            (&value_1, 1),
            (&value_2, 3),
            (&value_3, 6),
            (&value_4, 10),
            (&value_5, 15)
        ]
    );
    assert!(checker.is_active());

    subject.on_termination(Termination::Error(&error));
    assert_eq!(
        checker.values(),
        [
            (&value_1, 1),
            (&value_2, 3),
            (&value_3, 6),
            (&value_4, 10),
            (&value_5, 15)
        ]
    );
    assert!(checker.is_error(&error));
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(0, |last, value| last + value);

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(1);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(0, |last, value| last + value);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(1);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .scan(0, |last, value| last + value)
        .scan(1, |last, value| last * value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(1);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    subject.on_next(2);
    assert_eq!(checker.values(), [1, 3]);
    assert!(checker.is_active());

    subject.on_next(3);
    assert_eq!(checker.values(), [1, 3, 18]);
    assert!(checker.is_active());

    subject.on_next(4);
    assert_eq!(checker.values(), [1, 3, 18, 180]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [1, 3, 18, 180]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Scan::new(observable, 0, |last, value| last + value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(1);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    subject.on_next(2);
    assert_eq!(checker.values(), [1, 3]);
    assert!(checker.is_active());

    subject.on_next(3);
    assert_eq!(checker.values(), [1, 3, 6]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1, 3, 6]);
    assert!(checker.is_completed());
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.scan(0, |last, value| last + value);

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });
        let observable = observable.scan(&life_marker_2, |last, _| last);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.scan(0, |last, value| {
        s.consume_mut();
        last + value
    });

    observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.scan(0, |last, _| last);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let observable = subject.scan(0, |last, value| last + value);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let observable = subject.scan(0, |last, value| last + value);

    observable.buffer_with_count(1);
}
