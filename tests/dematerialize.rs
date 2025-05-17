mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Event, Observer, Termination},
    operators::{creating::create::Create, utility::dematerialize::Dematerialize},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::<Infallible>::Completed));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_completed());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222],));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_outer_finish() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222],));
    assert!(checker.is_completed());

    subject.on_next(Event::Termination(Termination::<Infallible>::Completed));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_concat_materialize() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize().materialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[Event::Next(111)]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[Event::Next(111), Event::Next(222)]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::<Infallible>::Completed));
    assert!(checker.is_values_matched(&[
        Event::Next(111),
        Event::Next(222),
        Event::Termination(Termination::Completed)
    ]));
    assert!(checker.is_completed());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[
        Event::Next(111),
        Event::Next(222),
        Event::Termination(Termination::Completed)
    ]));
    assert!(checker.is_completed());
}

#[test]
fn test_error_inner_finish() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::Error("error")));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_error("error"));

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222],));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_concat_materialize() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize().materialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[Event::Next(111)]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[Event::Next(111), Event::Next(222)]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::Error("error")));
    assert!(checker.is_values_matched(&[
        Event::Next(111),
        Event::Next(222),
        Event::Termination(Termination::Error("error"))
    ]));
    assert!(checker.is_completed());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[
        Event::Next(111),
        Event::Next(222),
        Event::Termination(Termination::Error("error"))
    ]));
    assert!(checker.is_completed());
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject.on_next(Event::Termination(Termination::Error("error")));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222,]));
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = 333;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(&value_1));
    assert!(checker.is_values_matched(&[&value_1]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(&value_2));
    assert!(checker.is_values_matched(&[&value_1, &value_2]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::Error(&error)));
    assert!(checker.is_values_matched(&[&value_1, &value_2,],));
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::new(|mut observer| {
        observer.on_next(Event::Next(&mut value));
        observer.on_next(Event::Termination(Termination::Error(&mut error)));
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.dematerialize();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(*error));
                *error *= 2;
            }
        },
    );

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(Event::Next(111));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_dropped());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(Event::Termination(Termination::Error("error")));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(Event::Termination(Termination::Error("error")));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_1.is_error("error"));
}

#[test]
fn test_multiple_operation_complete() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize().dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(Event::<_, &str>::Next(111)));

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::Completed));
    assert!(checker.is_values_matched(&[111,]));
    assert!(checker.is_completed());
}

#[test]
fn test_multiple_operation_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.dematerialize().dematerialize();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(Event::Next(111)));

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(Event::Termination(Termination::Error("error"))));
    assert!(checker.is_values_matched(&[111,]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Dematerialize::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(111));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(Event::Next(222));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_next(Event::Termination(Termination::<&str>::Completed));
    assert!(checker.is_values_matched(&[111, 222],));
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
            observer.on_next(Event::<_, &str>::Next(1));
            observer.on_termination(Termination::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.dematerialize();

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
        let observable = observable.dematerialize();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(Event::<_, TestStruct>::Next(TestStruct));
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.dematerialize();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Event<i32, &str>, _> = PublishSubject::default();
    let observable = subject.dematerialize();

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.dematerialize();

    observable.buffer_with_count(1);
}
