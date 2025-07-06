mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{
        Observable, connectable_observable::ConnectableObservable, observable_ext::ObservableExt,
    },
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    subject::replay_subject::ReplaySubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_completed_with_buffer_0() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(Some(0));
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_completed_with_buffer_1() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(Some(1));
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_error_with_buffer_0() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(Some(0));
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_error_with_buffer_1() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(Some(1));
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<_, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_dropped());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_dropped());
    assert!(channel_checker.is_subscribed());

    subscription.dispose();
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_dropped());
    assert!(channel_checker.is_unsubscribed());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_ref() {
    let mut counter = 0;
    let value_1 = 111;
    let value_2 = 222;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            match counter {
                1 => &value_1,
                2 => &value_2,
                _ => panic!(),
            }
        })
        .replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [&value_1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_async() {
    block_on(async {
        let mut counter = 0;
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = observable
            .map(move |_| {
                counter += 1;
                counter
            })
            .replay(None);
        let observable_1 = observable.clone();
        let observable_2 = observable.clone();

        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(channel_checker.is_initialized());

        let _subscription_1 = spawn(async move { observable_1.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(channel_checker.is_initialized());

        let _subscription = spawn(async move { observable.connect() }).await.unwrap();
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(channel_checker.is_subscribed());

        let mut sender = spawn(async move {
            sender.on_next(());
            sender
        })
        .await
        .unwrap();
        assert_eq!(checker_1.values(), [1]);
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(channel_checker.is_subscribed());

        let _subscription_2 = spawn(async move { observable_2.subscribe(observer_2) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [1]);
        assert!(checker_2.is_active());
        assert!(channel_checker.is_subscribed());

        let sender = spawn(async move {
            sender.on_next(());
            sender
        })
        .await
        .unwrap();
        assert_eq!(checker_1.values(), [1, 2]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [1, 2]);
        assert!(checker_2.is_active());
        assert!(channel_checker.is_subscribed());

        spawn(async { sender.on_termination(Termination::<Infallible>::Completed) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1, 2]);
        assert!(checker_1.is_completed());
        assert_eq!(checker_2.values(), [1, 2]);
        assert!(checker_2.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_multiple_operation() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.map(|_| {
        counter += 1;
        counter
    });
    let co_1 = observable.replay(None);
    let co_2 = co_1.clone().replay(None);
    let observable_1 = co_2.clone();
    let observable_2 = co_2.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription = co_1.connect();
    let _subscription = co_2.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.map(|_| {
        counter += 1;
        counter
    });
    let observable = ConnectableObservable::<_, ReplaySubject<'_, _, _>>::new(
        observable,
        ReplaySubject::new(None),
    );
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let _subscription_2 = observable.connect();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_share_api() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .share_replay(None);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2, 3]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2, 3]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription_1;

    // Error
    // let _subscription_1;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.replay(None);
        _subscription_1 = observable.clone().connect();

        let (_, observer) = Checker::<_, ()>::new();
        let _subscription_2 = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker = None;

    // Error
    // let mut life_marker = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker = Some(observer);
            Subscription::new_none_disposal()
        });
        let observable = observable.replay(None);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_2]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker);

        let observable = Create::new(|_: BoxedObserver<'_, &TestStruct, Infallible>| {
            Subscription::new_none_disposal()
        });
        let observable = observable.replay(None);
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.replay(None);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.replay(None);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.replay(None);

    observable.filter(|_| true);
}
