mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, utility::do_after_subscription::DoAfterSubscription},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

use crate::tests_utils::test_channel::test_channel;

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let mut called = false;

    // Custom operations
    let observable = observable.do_after_subscription(|| {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let _subscription = observable.subscribe(observer);
    assert!(called);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let mut called = false;

    // Custom operations
    let observable = observable.do_after_subscription(|| {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let _subscription = observable.subscribe(observer);
    assert!(called);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let mut called = false;

    // Custom operations
    let observable = observable.do_after_subscription(|| {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let subscription = observable.subscribe(observer);
    assert!(called);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let mut called = false;

    // Custom operations
    let observable = observable.do_after_subscription(|| {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let _subscription = observable.subscribe(observer);
    assert!(called);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_error(&error));
    assert!(channel_checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let (mut sender, observable, channel_checker) = test_channel();
    let mut called = false;

    // Custom operations
    let observable = observable.do_after_subscription(|| {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |_: Termination<Infallible>| {},
    );
    assert!(called);
    assert!(channel_checker.is_subscribed());

    sender.on_next(&mut value);
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert!(channel_checker.is_completed());

    drop(subscription);
    drop(channel_checker);
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let called = Arc::new(AtomicBool::new(false));

        // Custom operations
        let called_cloned = called.clone();
        let channel_checker_cloned = channel_checker.clone();
        let observable = observable.do_after_subscription(move || {
            called_cloned.store(true, Ordering::SeqCst);
            assert!(channel_checker_cloned.is_subscribed());
        });

        let handle = spawn(async move { observable.subscribe(observer) });
        let _subscription = handle.await.unwrap();
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(checker.values(), []);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let handle = spawn(async move {
            sender.on_next(111);
            sender
        });
        let sender = handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let handle = spawn(async move {
            sender.on_termination(Termination::<Infallible>::Completed);
        });
        handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_after_subscription(|| {});
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

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let mut called_1 = false;
    let mut called_2 = false;

    // Custom operations
    let observable = observable
        .do_after_subscription(|| {
            called_1 = true;
            assert!(channel_checker.is_subscribed());
        })
        .do_after_subscription(|| {
            called_2 = true;
            assert!(channel_checker.is_subscribed());
        });

    let _subscription = observable.subscribe(observer);
    assert!(called_1);
    assert!(called_2);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let mut called = false;

    // Custom operations
    let observable = DoAfterSubscription::new(observable, || {
        called = true;
        assert!(channel_checker.is_subscribed());
    });

    let _subscription = observable.subscribe(observer);
    assert!(called);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
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

        let observable = observable.do_after_subscription(|| {});

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
        let observable = observable.do_after_subscription(|| {});

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_after_subscription(|| {
        s.consume();
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
    let observable = observable.do_after_subscription(|| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_after_subscription(|| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_after_subscription(|| {});

    observable.filter(|_| true);
}
