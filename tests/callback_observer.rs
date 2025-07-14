mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use std::{convert::Infallible, time::Duration};

use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::{
    observable::observable_ext::ObservableExt,
    observer::{Observer, Termination},
    operators::creating::{create::Create, just::Just},
    subject::publish_subject::PublishSubject,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(on_next, on_termination);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(on_next, on_termination);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (on_next, on_termination) = observer_1.into_callbacks();
    let subscription_1 = observable_1.subscribe_with_callback(on_next, on_termination);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(on_next, on_termination);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let observable = Just::new(&mut value);
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_termination,
    );

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = runtime
            .clone()
            .spawn(async move {
                let (on_next, on_termination) = observer.into_callbacks();
                observable.subscribe_with_callback(on_next, on_termination)
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(&111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .clone()
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_termination) = observer_1.into_callbacks();
    let _subscription_1 = observable
        .clone()
        .subscribe_with_callback(on_next, on_termination);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
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

        _subscription = observable.subscribe_with_callback(|_| {}, |_| {});
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

        let on_next = |_: i32| life_marker_2.consume_ref();
        let on_termination = |_: Termination<String>| life_marker_2.consume_ref();
        let _subscription = observable.subscribe_with_callback(on_next, on_termination);
    }
}

#[test]
fn test_fn() {
    let mut s1 = TestStruct;
    let s2 = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.subscribe_with_callback(
        |_| {
            s1.consume_mut();
        },
        |_| {
            s2.consume();
        },
    );
}
