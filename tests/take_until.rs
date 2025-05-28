mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{conditional_boolean::take_until::TakeUntil, creating::create::Create},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let _stop_subject: PublishSubject<'_, (), _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(_stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let _stop_subject: PublishSubject<'_, (), _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(_stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_completed_stop_next() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    stop_subject.on_next(());
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());

    stop_subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_completed_stop_completed() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let stop_subject: PublishSubject<'_, Infallible, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    stop_subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_error_stop_error() {
    let mut subject = PublishSubject::default();
    let stop_subject: PublishSubject<'_, Infallible, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    stop_subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_error("error"));

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_same_source_stop_next() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_completed());

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert!(checker.is_completed());

    subject.on_termination(Termination::<&str>::Completed);
    assert!(checker.values().is_empty());
    assert!(checker.is_completed());
}

#[test]
fn test_same_source_stop_completed() {
    let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert!(checker.is_completed());
}

#[test]
fn test_same_source_stop_error() {
    let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    stop_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());

    stop_subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let mut subject = PublishSubject::default();
    let stop_subject: PublishSubject<'_, Infallible, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(&value_1);
    assert_eq!(checker.values(), [&value_1]);
    assert!(checker.is_active());

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert!(checker.is_active());

    stop_subject.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
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
        Subscription::new_none_disposal()
    });

    let stop_subject: PublishSubject<'_, Infallible, _> = PublishSubject::default();
    let observable = observable.take_until(stop_subject.clone());

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => unreachable!(),
            Termination::Error(error) => assert_eq!(error, "error"),
        },
    );

    stop_subject.on_termination(Termination::Error("error"));

    drop(subscription);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let handle = tokio::spawn(async move {
        stop_subject.on_next(());
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.take_until(stop_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    stop_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_multiple_operation_stop_1() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let mut stop_subject_1 = PublishSubject::default();
    let stop_subject_2: PublishSubject<'_, (), Infallible> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .take_until(stop_subject_1.clone())
        .take_until(stop_subject_2.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    stop_subject_1.on_next(());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_multiple_operation_stop_2() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let stop_subject_1: PublishSubject<'_, (), Infallible> = PublishSubject::default();
    let mut stop_subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .take_until(stop_subject_1.clone())
        .take_until(stop_subject_2.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    stop_subject_2.on_next(());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_multiple_operation_same_stop() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .take_until(stop_subject.clone())
        .take_until(stop_subject.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    stop_subject.on_next(());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let mut stop_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = TakeUntil::new(observable, stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    stop_subject.on_next(());
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());

    stop_subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_unsub_after_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut stop_sender, stop_observable, stop_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(stop_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(stop_channel_checker.is_subscribed());

    stop_sender.on_next(());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
    assert!(stop_channel_checker.is_unsubscribed());
}

#[test]
fn test_unsub_after_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(stop_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(stop_channel_checker.is_subscribed());

    stop_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_unsubscribed());
    assert!(stop_channel_checker.is_error("error"));
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker_1 = TestStruct;
    let life_marker_2 = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new_with_disposal_callback(|| {
                life_marker_1.consume_ref();
            })
        });
        let stop_subject = Create::new(|mut observer| {
            observer.on_next(());
            Subscription::new_with_disposal_callback(|| {
                life_marker_2.consume_ref();
            })
        });
        let observable = observable.take_until(stop_subject);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_3 = TestStruct;
    let mut life_marker_1 = None;
    let mut life_marker_2 = None;

    // Error
    // let mut life_marker_1 = None;
    // let mut life_marker_2 = None;
    // let life_marker_3 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });
        let stop_subject = Create::new(|observer: BoxedObserver<'_, (), _>| {
            life_marker_2 = Some(observer);
            Subscription::new_none_disposal()
        });
        let observable = observable.take_until(stop_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_3]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let stop_subject = Create::new(|_: BoxedObserver<'_, (), _>| Subscription::new_none_disposal());
    let observable = observable.take_until(stop_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let stop_subject: PublishSubject<'_, (), _> = PublishSubject::default();
    let observable = subject.take_until(stop_subject);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let stop_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.take_until(stop_subject);

    observable.buffer_with_count(1);
}
