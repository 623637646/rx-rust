mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Terminal},
    operators::{creating::create::Create, transforming::buffer_with_count::BufferWithCount},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(666);
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
    assert!(checker.is_completed());
}

#[test]
fn test_error_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(666);
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_one_count() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[vec![111]]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[vec![111], vec![222]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[vec![111], vec![222]]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(3);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(333);
    assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_2.is_active());

    subject.on_next(444);
    assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();

    subject.on_next(555);
    assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_2.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(&value_1);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(&value_2);
    assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
    assert!(checker.is_active());

    subject.on_next(&value_3);
    assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error(&error));
    assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value_1);
        observer.on_next(&mut value_2);
        observer.on_next(&mut value_3);
        observer.on_terminal(Terminal::Error("error"));
        Subscription::new_none_disposal()
    });
    let observable = observable.buffer_with_count(2);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            for i in value {
                *i *= 2;
            }
        },
        |terminal| assert!(matches!(terminal, Terminal::Error("error"))),
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 333);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(2);

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![111, 222]]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(333);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![111, 222]]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![111, 222]]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[vec![111, 222]]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_terminal) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[vec![111, 222]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![111, 222]]));
    assert!(checker_2.is_active());

    subject.on_next(333);
    assert!(checker_1.is_values_matched(&[vec![111, 222]]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[vec![111, 222]]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[vec![111, 222]]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[vec![111, 222]]));
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(2).buffer_with_count(2);

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
    assert!(checker.is_active());

    subject.on_next(666);
    assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
    assert!(checker.is_active());

    subject.on_next(777);
    assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[
        vec![vec![111, 222], vec![333, 444]],
        vec![vec![555, 666], vec![777]]
    ]));
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = BufferWithCount::new(observable, 3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(222);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(333);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(444);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.on_next(555);
    assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
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
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.buffer_with_count(2).buffer_with_count(2);

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
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });
        let observable = observable.buffer_with_count(2);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_2]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_terminal(Terminal::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.buffer_with_count(3);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(3);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(3);

    _ = observable.buffer_with_count(1);
}
