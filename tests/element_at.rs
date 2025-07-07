mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, sleep, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, filtering::element_at::ElementAt},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), []);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_completed_reach_index() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker.values(), [333]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_completed_0_index() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(0);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
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
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subscription.dispose();
    assert_eq!(checker.values(), []);
    assert!(checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value_2);
    assert_eq!(checker.values(), [&value_2]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;

    let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, Infallible>();

    // Custom operations
    let observable = observable.element_at(1);

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
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
    assert!(channel_checker.is_unsubscribed());

    drop(sender);
    drop(channel_checker);
    drop(subscription);

    assert_eq!(value_1, 111);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.element_at(10);

        let subscription = spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let mut sender = spawn(async move {
            sender.on_next(111);
            sender
        })
        .await
        .unwrap();
        assert_eq!(checker.values(), []);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        spawn(async move {
            sender.on_next(222);
        })
        .await
        .unwrap();
        assert_eq!(checker.values(), []);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        spawn(async { subscription.dispose() }).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.element_at(1);
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
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [222]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_multiple_operation_1_0() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(1).element_at(0);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker.values(), [222]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_multiple_operation_0_1() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.element_at(0).element_at(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = ElementAt::new(observable, 2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), []);
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
            observer.on_next(111);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.element_at(2);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
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
        let observable = observable.element_at(2);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_2]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub = TestStruct;
    let mut life_marker_or = None;

    // Error
    // let mut life_marker_or = None;
    // let life_marker_sub = TestStruct;

    {
        let observable = Create::new(|observer: BoxedObserver<'_, &TestStruct, Infallible>| {
            life_marker_or = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                life_marker_sub.consume_ref();
            })
        });

        let observable = observable.element_at(2);

        let (_, observer) = Checker::new();
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
    let observable = observable.element_at(2);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.element_at(2);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.element_at(2);

    observable.filter(|_| true);
}
