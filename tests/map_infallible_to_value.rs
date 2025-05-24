mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, others::map_infallible_to_value::MapInfallibleToValue},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, _>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::<i32, String>::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.clone().on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let error = 111;

    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<&i32, _>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Error(&error));
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut error = 111;

    // Custom operations
    let observable = Create::new(|observer| {
        observer.on_termination(Termination::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let observable = observable.map_infallible_to_value();

    let _subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |termination| {
            match termination {
                Termination::Completed => unreachable!(),
                Termination::Error(error) => {
                    *error *= 2;
                }
            };
        },
    );

    assert_eq!(error, 222);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<&i32, String>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Completed);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::<i32, String>::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_value();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.clone().on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_multiple_operation() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map_infallible_to_value()
        .map_infallible_to_value();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = MapInfallibleToValue::new(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[]));
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
        let observable = Create::new(|_| {
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.map_infallible_to_value();

        let (_, observer) = Checker::<i32, String>::new();
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
        let observable = observable.map_infallible_to_value();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
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
    let observable = observable.map_infallible_to_value::<String>();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject = PublishSubject::default();
    let observable = subject.map_infallible_to_value();

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::<Vec<i32>, String>::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, _, String> = PublishSubject::default();
    let observable = subject.map_infallible_to_value::<String>();

    observable.buffer_with_count(1);
}
