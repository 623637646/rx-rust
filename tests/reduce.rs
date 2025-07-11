mod tests_utils;

use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, just::Just},
        mathematical_aggregate::reduce::Reduce,
    },
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::convert::Infallible;
use std::time::Duration;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.reduce(0, |last, value| last + value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(3);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [6]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.reduce(1, |last, value| last * value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(3);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.reduce(0, |last, value| last + value);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(1);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_next(2);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_next(3);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_next(4);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [10]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let value_0 = 0;
    let value_1 = 1;
    let value_2 = 2;
    let value_3 = 3;
    let value_4 = 4;
    let value_5 = 5;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.reduce((&value_0, 0), |last, value| (value, last.1 + *value));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_1);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_3);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_4);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_5);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(&value_5, 15)]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_mut_ref() {
    let mut value_0 = 0;
    let mut value_1 = 1;
    let mut value_2 = 2;
    let mut value_3 = 3;

    let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, _>();

    // Custom operations
    let observable = observable.reduce((&mut value_0, 0), |last, value| {
        let v = *value;
        (value, last.1 + v)
    });

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value.0 *= 3;
            assert_eq!(value.1, 6);
        },
        |termination| match termination {
            Termination::Completed => {}
            Termination::Error(_) => unreachable!(),
        },
    );
    assert!(channel_checker.is_subscribed());

    sender.on_next(&mut value_1);
    assert!(channel_checker.is_subscribed());

    sender.on_next(&mut value_2);
    assert!(channel_checker.is_subscribed());

    sender.on_next(&mut value_3);
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert!(channel_checker.is_completed());

    drop(channel_checker);
    drop(subscription);

    assert_eq!(value_0, 0);
    assert_eq!(value_1, 1);
    assert_eq!(value_2, 2);
    assert_eq!(value_3, 9);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.reduce(0, |last, value| last + value);

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let _sender = runtime
            .spawn(async move {
                sender.on_next(1);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.reduce(0, |last, value| last + value);
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
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .reduce(0, |last, value| last + value)
        .reduce(1, |last, value| last * value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(3);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(4);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [10]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Reduce::new(observable, 0, |last, value| last + value);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(3);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [6]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
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

        let observable = observable.reduce(0, |last, value| last + value);

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
        let observable = observable.reduce(&life_marker_2, |last, _| last);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    // Custom operations
    let observable = Just::new(1).reduce(0, |last, value| {
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
    let observable = observable.reduce(0, |last, _| last);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Just::new(1).reduce(0, |last, value| last + value);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Just::new(1).reduce(0, |last, value| last + value);

    observable.filter(|_| true);
}
