mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Terminal},
    operators::{creating::create::Create, transforming::map::Map},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| value.to_string());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_completed());

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_error() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| value.to_string());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_error("error"));

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_unsubscribe() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| value.to_string());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&["111".to_owned()]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&["111".to_owned()]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&["111".to_owned(), "222".to_owned()]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&["111".to_owned(), "222".to_owned()]));
    assert!(checker_2.is_error("error"));

    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_2_ref = &value_2;
    let error = 333;

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, mut observer_2) = Checker::<_, &str>::new();

    let mut subject = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(move |value| {
        observer_2.on_next(value);
        value_2_ref
    });

    let subscription = observable.subscribe(observer_1);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(&value_1);
    assert!(checker_1.is_values_matched(&[&value_2]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[&value_1]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error(&error));
    assert!(checker_1.is_values_matched(&[&value_2]));
    assert!(checker_1.is_error(&error));
    assert!(checker_2.is_values_matched(&[&value_1]));
    assert!(checker_2.is_dropped());

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value);
        observer.on_terminal(Terminal::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.map(|value| {
        *value *= 2;
        (value.to_string(), value)
    });

    let (mut on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(
        |value| {
            on_next(value.0);
            *value.1 *= 2;
        },
        |terminal| match terminal {
            Terminal::Completed => panic!(),
            Terminal::Error(error) => {
                on_terminal(Terminal::Error(*error));
                *error *= 2;
            }
        },
    );

    assert!(checker.is_values_matched(&["222".to_owned()]));
    assert!(checker.is_error(222));
    assert_eq!(value, 444);
    assert_eq!(error, 444);

    _ = subscription; // keep the subscription alive
}

#[tokio::test]
async fn test_async() {
    let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| value.to_string());

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| value.to_string());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&["111".to_owned()]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&["111".to_owned()]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&["111".to_owned()]));
    assert!(checker_2.is_error("error"));

    _ = subscription_1; // keep the subscription alive
    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_multiple_operation() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|value| value.to_string())
        .map(|value| value + "?");

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&["111?".to_owned()]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&["111?".to_owned()]));
    assert!(checker.is_error("error"));

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_without_convenient_api() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Map::new(observable, |value| value.to_string());

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&["111".to_owned()]));
    assert!(checker.is_error("error"));

    _ = subscription; // keep the subscription alive
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

        let observable = observable.map(|value| value.to_string());

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
        let observable = observable.map(|_: i32| None);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker_2));
        let subscription = observable.subscribe(observer);

        _ = subscription; // keep the subscription alive
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.map(|value| {
        s.consume_mut();
        value.to_string()
    });

    observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_terminal(Terminal::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.map(|value| value);
    let _ = observable.clone(); // make sure it's Clone when T and E is not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.map(|value| value.to_string());

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.map(|value| value.to_string());

    let _ = observable.buffer_with_count(1);
}
