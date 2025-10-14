mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, never::Never},
        utility::delay::Delay,
    },
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_then_error() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        subject.clone().on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(444);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        let _subscription_3 = observable_3.subscribe(observer_3);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        subject.on_next(111);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111]);
        assert_eq!(checker_3.state(), State::Active);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped); // This assert is ok in multi-threaded because the scheduler is finished.
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111]);
        assert_eq!(checker_3.state(), State::Active);

        subject.on_next(222);
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111]);
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111]);
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111, 222]);
        assert_eq!(checker_3.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111, 222]);
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(checker_3.values(), [111, 222]);
        assert_eq!(checker_3.state(), State::Active);

        subscription_2.dispose();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        // assert_eq!(checker_2.state(), State::Active); // This assert may be failed in multi-threaded.
        assert_eq!(checker_3.values(), [111, 222]);
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Dropped);
        assert_eq!(checker_3.values(), [111, 222, 333]);
        assert_eq!(checker_3.state(), State::Active);

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Dropped);
        assert_eq!(checker_3.values(), [111, 222, 333]);
        assert_eq!(checker_3.state(), State::Error("error"));
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = runtime
            .spawn(async move {
                sender.on_next(&111);
                sender
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
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
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Error("error"));
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Error("error"));
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone()).take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .delay(DURATION_100_MS, runtime.clone())
            .delay(DURATION_100_MS + DURATION_30_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_100_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Delay::new(observable, DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(444);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_completed() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        sender.on_termination(Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_unsub() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        subscription.dispose();
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            sender.on_next(*i);
        }
        sender.on_termination(Termination::<Infallible>::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_immediate_next() {
    block_on(|runtime| async move {
        let subject = BehaviorSubject::new(111);
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone().delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_immediate_completed() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Empty.delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_immediate_error() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Throw::new("error").delay(DURATION_100_MS, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_lifetime_sub() {
    block_on(|runtime| async move {
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

            let observable = observable.delay(DURATION_100_MS, runtime.clone());

            let (_, observer) = Checker::new();
            _subscription = observable.subscribe(observer);
        }

        runtime.sleep(DURATION_30_MS * 2).await;
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.delay(DURATION_100_MS, runtime.clone());
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.delay(DURATION_100_MS, runtime.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.delay(DURATION_100_MS, runtime.clone());

        observable.filter(|_| true);
    });
}
