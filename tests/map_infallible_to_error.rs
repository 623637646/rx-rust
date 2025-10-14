mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, others::map_infallible_to_error::MapInfallibleToError},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_error();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::<_, String>::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_error();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
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

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value = 111;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::<&i32, String>::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_error();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Completed);
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
        observer.on_termination(Termination::Completed);
        Subscription::default()
    });
    let observable = observable.map_infallible_to_error();

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination: Termination<String>| assert!(matches!(termination, Termination::Completed)),
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::<&i32, String>::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(&111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::<_, String>::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone().map_infallible_to_error();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

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

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = observable.map_infallible_to_error().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::<_, String>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map_infallible_to_error()
        .map_infallible_to_error();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::<i32, String>::new();

    // Custom operations
    let observable = MapInfallibleToError::new(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
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
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.map_infallible_to_error();

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
            Subscription::default()
        });
        let observable = observable.map_infallible_to_error();

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
        Subscription::default()
    });
    let observable = observable.map_infallible_to_error::<String>();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
    let observable = subject.map_infallible_to_error();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::<_, String>::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
    let observable = subject.map_infallible_to_error::<String>();

    observable.filter(|_| true);
}
