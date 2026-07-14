mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observer::boxed_observer::BoxedObserver;
use rx_rust::operators::conditional_boolean::contains::Contains;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::creating::{create::Create, just::Just},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_true() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(888);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(888);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_false() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(888);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [false]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(999);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(888);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(999);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(888);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let (checker, observer) = Checker::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();

    // Custom operations
    let observable = observable.contains(&value_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value_1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value_2);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.contains(333);

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move { sender.on_next(333) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [true]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.contains(222);
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
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [false]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [false]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(777).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(777);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_positive_true() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(666).contains(true);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_positive_false() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(666).contains(false);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), [false]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_negative_true() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(888).contains(false);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_multiple_operation_negative_false() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.contains(888).contains(true);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [false]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Contains::new(observable, 666);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(666);
    assert_eq!(checker.values(), [true]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.contains(1);

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = 111;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = 111;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.contains(&life_marker_2);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(true);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub = 111;
    let mut life_marker_or = None;

    // Error
    // let mut life_marker_or = None;
    // let life_marker_sub = 111;

    {
        let observable = Create::new(|observer: BoxedObserver<'_, &i32, Infallible>| {
            life_marker_or = Some(observer);
            Subscription::new(CallbackDisposal::new(|| {
                let _ = life_marker_sub;
            }))
        });

        let observable = observable.contains(&life_marker_sub);

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.contains(111);
    _ = observable.clone(); // Make sure it's Clone when E is not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Just::new(1).contains(111);

    let observable = observable.contains(true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Just::new(1).contains(111);

    observable.contains(true);
}
