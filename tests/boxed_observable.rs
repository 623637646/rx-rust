mod tests_utils;

use rx_rust::{
    observable::{Observable, boxed_observable::BoxedObservable, observable_ext::ObservableExt},
    observer::{Observer, Terminal},
    operators::creating::{create::Create, just::Just},
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
    let observable = BoxedObservable::new(subject.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = BoxedObservable::new(subject.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_error("error"));

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = BoxedObservable::new(subject.clone());
    let observable_2 = BoxedObservable::new(subject.clone());

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_error("error"));

    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = BoxedObservable::new(subject.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(&value);
    assert!(checker.is_values_matched(&[&value]));
    assert!(checker.is_active());

    subject.clone().on_terminal(Terminal::Error(&error));
    assert!(checker.is_values_matched(&[&value]));
    assert!(checker.is_error(&error));

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let observable = Just::new(&mut value);
    let observable = BoxedObservable::new(observable);

    let (checker, observer) = Checker::new();

    let (mut on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_terminal,
    );

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
    assert_eq!(value, 222);

    _ = subscription; // keep the subscription alive
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = BoxedObservable::new(subject.clone());

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(&111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = BoxedObservable::new(subject.clone());
    let observable_2 = BoxedObservable::new(subject.clone());

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_error("error"));

    _ = subscription_1; // keep the subscription alive
    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_lifetime_sub() {
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

        let observable = BoxedObservable::new(observable);

        let (_, observer) = Checker::new();
        subscription = observable.subscribe(observer);
    }

    _ = subscription; // keep the subscription alive
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
        let observable = BoxedObservable::new(observable);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker_2));
        let subscription = observable.subscribe(observer);

        _ = subscription; // keep the subscription alive
    }
}

#[test]
fn test_lifetime_oe() {
    // OK
    let life_marker = TestStruct;
    let observable;

    // Error
    // let observable;
    // let life_marker = TestStruct;

    {
        let create = Create::new(|mut observer| {
            life_marker.consume_ref();
            observer.on_next(1);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });

        observable = BoxedObservable::new(create);
    }

    _ = observable;
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = BoxedObservable::new(subject);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = BoxedObservable::new(subject);

    let _ = observable.buffer_with_count(1);
}
