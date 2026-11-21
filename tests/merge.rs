mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{combining::merge::Merge, creating::just::Just},
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().merge(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111, 111]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 111, 222, 222]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 111, 222, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_error_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().merge(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111, 111]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 111, 222, 222]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 111, 222, 222]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_first_is_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (channels_1, source_1) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.merge(source_1);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 222).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(0, 333).is_continue());
    assert!(channels_1.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 444).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(1, 555).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    channels_1.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    let (mut sender, observable, _) = test_channel::<'_, &mut i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel();

    let observable = observable.merge(observable_1);

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |_| panic!(),
    );

    assert!(sender.on_next(&mut value_1).is_continue());
    assert!(sender.on_next(&mut value_2).is_continue());
    assert!(sender_1.on_next(&mut value_3).is_continue());

    drop(subscription);
    drop(sender);
    drop(sender_1);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.merge(observable_1);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _another_source_sender = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(222).is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (another_source_channels, another_source_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.merge(another_source_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    assert!(another_source_channels.on_next(0, 222).is_continue());
    assert!(another_source_channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    another_source_channels.on_termination(0, Termination::<Infallible>::Completed);
    another_source_channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Completed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(another_source_channels.state(0), ChannelState::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(another_source_channels.state(1), ChannelState::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (_sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge(observable_1).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .merge(observable_1)
        .merge(observable_2)
        .merge(observable_3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    sender_2.on_termination(Termination::<Infallible>::Completed);
    sender_3.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
}

#[test]
fn test_multiple_operation_same_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut another_source_subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = observable
        .merge(another_source_subject.clone())
        .merge(another_source_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111, 111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 111, 222, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 111, 222, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    another_source_subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 111, 222, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 111, 222, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Merge::new((observable, observable_1));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut sender_1, source_1, _) = test_channel();
    let source_1 = source_1.start_with([222]);
    let (checker, observer) = Checker::new();

    let observable = source.merge(source_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Empty.merge(Empty);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").merge(observable_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.merge(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.merge(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<i32, &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.merge(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker_1 = TestStruct;
    let life_marker_2 = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let another_source_subject = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(222).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = observable.merge(another_source_subject);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_3 = TestStruct;
    let mut life_marker_1 = None;
    let mut life_marker_2 = None;

    // Error
    // let mut life_marker_1 = None;
    // let mut life_marker_2 = None;
    // let life_marker_3 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let another_source_subject = Create::shared_boxed(|observer| {
            life_marker_2 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.merge(another_source_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(
            observer
                .on_next((&life_marker_3, &life_marker_3))
                .is_continue()
        );
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub_1 = TestStruct;
    let life_marker_sub_2 = TestStruct;

    let mut life_marker_or_1 = None;
    let mut life_marker_or_2 = None;

    // Error
    // let mut life_marker_or_1 = None;
    // let mut life_marker_or_2 = None;

    // let life_marker_sub_1 = TestStruct;
    // let life_marker_sub_2 = TestStruct;

    {
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                life_marker_or_1 = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_1.consume_ref();
                }))
            });

        let boundary = Create::shared_boxed(|observer| {
            life_marker_or_2 = Some(observer);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_sub_2.consume_ref();
            }))
        });

        let observable = observable.merge(boundary);

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let another_source_subject = Create::shared_boxed(|_| DisposeOnDrop::default());
    let observable = observable.merge(another_source_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel();
    let observable = observable.merge(another_source_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel();
    let observable = observable.merge(another_source_observable);

    observable.filter(|_| true);
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// the other one goes on. The abandoned one never completed, so neither does the result: once the
/// other completes, the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.merge(observable_2).subscribe(observer);
    assert!(sender_1.on_next(111).is_continue());
    sender_1.abandon();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_no_second_subscription_after_downstream_stops() {
    let (_, first, first_checker) = test_channel::<'_, i32, Infallible>();
    let (_, second, second_checker) = test_channel();
    let (checker, observer) = Checker::stopping_after(1);
    let _subscription = first.start_with([111]).merge(second).subscribe(observer);

    assert_eq!(first_checker.state(), ChannelState::Initialized);
    assert_eq!(second_checker.state(), ChannelState::Initialized);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_disposal_order() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let first_order = order.clone();
    let first = Create::shared_boxed(|_: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            first_order.with_mut(|order| order.push(1));
        }))
    });
    let second_order = order.clone();
    let second = Create::shared_boxed(|_| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            second_order.with_mut(|order| order.push(2));
        }))
    });
    let (_, observer) = Checker::new();
    let subscription = first.merge(second).subscribe(observer);

    subscription.dispose();
    assert_eq!(order.clone_value(), [2, 1]);
}

#[test]
fn test_completed_three_sources() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Sources of different types, merged without boxing.
    let _subscription = Merge::new((
        observable_1,
        observable_2.map(|value| value * 10),
        Just::new(100),
    ))
    .subscribe(observer);
    assert_eq!(checker.values(), [100]);

    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_2.on_next(2).is_continue());
    assert_eq!(checker.values(), [100, 1, 20]);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsubscribe_order_three_sources() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let order = order.clone();
        Create::shared_boxed(move |_observer: SendBoxedObserver<'_, i32, Infallible>| {
            let order = order.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
            }))
        })
    };

    let subscription =
        Merge::new((source(0), source(1), source(2))).subscribe_with_callback(|_| {}, |_| {});
    subscription.dispose();
    // The last subscribed first.
    assert_eq!(order.clone_value(), [2, 1, 0]);
}

#[test]
fn test_error_on_sub_skips_later_sources() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::<_, &str>::new();

    // The first one fails as it is subscribed: the others are not subscribed at all.
    let _subscription = Merge::new((
        Throw::new("error").with_item_type::<i32>().into_shared(),
        observable,
    ))
    .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_completed_from_iter() {
    let (checker, observer) = Checker::new();
    let _subscription = rx_rust::operators::combining::merge::Merge::new_from_iter(
        (0..12).map(|value| Just::new(value).into_shared()),
    )
    .subscribe(observer);
    // A synchronously completing source must not end enumeration before the next source.
    assert_eq!(checker.values(), (0..12).collect::<Vec<_>>());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_next_from_iter() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Merge::new_from_iter([observable_1, observable_2]);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<i32>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_error_from_iter() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Merge::new_from_iter([observable_1, observable_2]);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
}

#[test]
fn test_stop_from_iter() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let generated = AtomicUsize::new(0);
    let sources = (0..).map(|value| {
        generated.fetch_add(1, Ordering::SeqCst);
        Just::new(value).into_shared()
    });
    let (checker, observer) = Checker::new();
    let _subscription = rx_rust::operators::combining::merge::Merge::new_from_iter(sources)
        .take(1)
        .subscribe(observer);
    assert_eq!(generated.load(Ordering::SeqCst), 1);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_empty_from_iter() {
    let (checker, observer) = Checker::new();
    let _subscription =
        rx_rust::operators::combining::merge::Merge::new_from_iter(std::iter::empty::<Just<i32>>())
            .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_order_on_sub_panic() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    for from_iter in [false, true] {
        let order = Arc::new(Mutex::new(Vec::new()));
        // The source given the `PanicOnDrop` unwinds as it is subscribed.
        let source = |index, panic_on_drop: Option<PanicOnDrop>| {
            let order = order.clone();
            Create::shared_boxed(move |observer| {
                drop(panic_on_drop);
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    drop(observer);
                    order.with_mut(|order| order.push(index));
                }))
            })
            .with_item_type::<i32>()
            .with_error_type::<Infallible>()
        };
        expect_panic_on_drop(|panic_on_drop| {
            let last = source(3, Some(panic_on_drop));
            if from_iter {
                let sources = (0..3).map(|index| source(index, None)).chain([last]);
                let _subscription =
                    Merge::new_from_iter(sources).subscribe_with_callback(|_| {}, |_| {});
            } else {
                let sources = (source(0, None), source(1, None), source(2, None), last);
                let _subscription = Merge::new(sources).subscribe_with_callback(|_| {}, |_| {});
            }
        });
        assert_eq!(order.clone_value(), [2, 1, 0], "from_iter = {from_iter}");
    }
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_order_on_synchronous_completion_panic() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let order = Arc::new(Mutex::new(Vec::new()));
    let source = |index| {
        let order = order.clone();
        Create::shared_boxed(move |observer| {
            observer.on_termination(Termination::<Infallible>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
            }))
        })
        .with_item_type::<i32>()
    };
    expect_panic_on_drop(|panic_on_drop| {
        let _subscription = Merge::new((source(0), source(1), source(2)))
            .subscribe_with_callback(|_| {}, move |_| drop(panic_on_drop));
    });
    // Each source is disposed as it completes, the last one although the completion of the
    // stream panics.
    assert_eq!(order.clone_value(), [0, 1, 2]);
}

#[test]
fn test_release_on_completion() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let observers = Arc::new(Mutex::new(Vec::new()));
    let source = |index| {
        let order = order.clone();
        let observers = observers.clone();
        Create::shared_boxed(move |observer| {
            observers.with_mut(|observers| observers.push(Some(observer)));
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
            }))
        })
        .with_item_type::<i32>()
        .with_error_type::<Infallible>()
    };
    let (checker, observer) = Checker::new();
    let subscription = Merge::new((source(0), source(1), source(2))).subscribe(observer);
    assert!(order.clone_value().is_empty());

    // The subscription of a source that completed is disposed at once: the merge holds only
    // those of the sources still running.
    let second = observers.with_mut(|observers| observers[1].take().unwrap());
    second.on_termination(Termination::Completed);
    assert_eq!(order.clone_value(), [1]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(order.clone_value(), [1, 2, 0]);
}

#[test]
fn test_stop_during_iterator_next_skips_subscription() {
    let (channels, source) = test_channels::<'_, i32, Infallible>();
    let sources = (0..).map(|index| {
        if index == 1 {
            assert!(channels.on_next(0, 42).is_stop());
        }
        source.clone()
    });
    let (checker, observer) = Checker::new();
    let _subscription = Merge::new_from_iter(sources).take(1).subscribe(observer);
    assert_eq!(checker.values(), [42]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.len(), 1);
}

mod max_concurrency {
    use crate::tests_utils::checker::{Checker, State};
    use crate::tests_utils::test_channel::{ChannelState, test_channel, test_channels};
    use crate::tests_utils::test_scheduler::block_on;
    use crate::tests_utils::thread_checker_scheduler::{ThreadCheckerExecutor, get_thread_name};
    use rx_rust::disposable::Disposable;
    use rx_rust::disposable::callback_disposal::CallbackDisposal;
    use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
    use rx_rust::observer::boxed_observer::SendBoxedObserver;
    use rx_rust::operators::creating::create::Create;
    use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::{
            combining::merge::Merge,
            creating::{empty::Empty, just::Just},
        },
    };
    use std::convert::Infallible;
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};

    const ONE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    const TWO: NonZeroUsize = NonZeroUsize::new(2).unwrap();
    const THREE: NonZeroUsize = NonZeroUsize::new(3).unwrap();

    /// A function from an index to a source that records its subscription and its disposal in
    /// `$log`, as `index` and `-index`, and keeps its observer in `$observers`. It owns handles to
    /// both, as an observer of a merge names the type of every source it pulls.
    macro_rules! logged_source {
        ($log:expr, $observers:expr) => {{
            let (log, observers) = ($log.clone(), $observers.clone());
            move |index: i32| {
                let (log, observers) = (log.clone(), observers.clone());
                Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
                    log.with_mut(|log| log.push(index));
                    observers.with_mut(|observers| observers.push(Some(observer)));
                    let log = log.clone();
                    DisposeOnDrop::new(CallbackDisposal::new(move || {
                        log.with_mut(|log| log.push(-index));
                    }))
                })
            }
        }};
    }

    #[test]
    fn test_completed() {
        let (sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
        let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Sources of different types, the third pulled once there is room for it.
        let _subscription = Merge::new((
            observable_1,
            observable_2.map(|value| value * 10),
            observable_3,
        ))
        .max_concurrency(TWO)
        .subscribe(observer);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_3.state(), ChannelState::Initialized);

        assert!(sender_2.on_next(2).is_continue());
        assert_eq!(checker.values(), [20]);

        sender_1.on_termination(Termination::Completed);
        assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
        assert_eq!(checker.state(), State::Active);

        assert!(sender_3.on_next(3).is_continue());
        assert_eq!(checker.values(), [20, 3]);

        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Active);
        sender_3.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [20, 3]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_from_iter() {
        let (checker, observer) = Checker::new();
        let _subscription = Merge::new_from_iter([Just::new(1), Just::new(2), Just::new(3)])
            .max_concurrency(TWO)
            .subscribe(observer);
        assert_eq!(checker.values(), [1, 2, 3]);
        assert_eq!(checker.state(), State::Completed::<Infallible>);
    }

    #[test]
    fn test_completed_empty_from_iter() {
        let (checker, observer) = Checker::new();
        let _subscription = Merge::new_from_iter(std::iter::empty::<Just<i32>>())
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed::<Infallible>);
    }

    #[test]
    fn test_completed_with_synchronous_chain() {
        let (checker, observer) = Checker::new();
        // Every source completes as it is subscribed: they are pulled in a loop, not by recursion.
        let _subscription = Merge::new_from_iter((0..10_000).map(Just::new))
            .max_concurrency(ONE)
            .subscribe(observer);
        assert_eq!(checker.values(), (0..10_000).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed::<Infallible>);
    }

    #[test]
    fn test_pulls_as_room_is_made() {
        let (channels, source) = test_channels::<'_, i32, Infallible>();
        let pulled = Arc::new(AtomicUsize::new(0));
        // An iterator that never ends: only what there is room for is pulled.
        let sources = {
            let pulled = pulled.clone();
            (0..).map(move |_| {
                pulled.fetch_add(1, Ordering::SeqCst);
                source.clone()
            })
        };
        let (checker, observer) = Checker::new();

        let subscription = Merge::new_from_iter(sources)
            .max_concurrency(TWO)
            .subscribe(observer);
        assert_eq!(pulled.load(Ordering::SeqCst), 2);
        assert_eq!(channels.len(), 2);

        assert!(channels.on_next(1, 11).is_continue());
        assert_eq!(checker.values(), [11]);

        channels.on_termination(0, Termination::Completed);
        assert_eq!(pulled.load(Ordering::SeqCst), 3);
        assert_eq!(channels.len(), 3);
        assert_eq!(checker.state(), State::Active);

        assert!(channels.on_next(2, 22).is_continue());
        assert_eq!(checker.values(), [11, 22]);

        subscription.dispose();
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
        assert_eq!(channels.state(2), ChannelState::Unsubscribed);
        assert_eq!(pulled.load(Ordering::SeqCst), 3);
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_room_made_while_pulling() {
        let (channels, source) = test_channels::<'_, i32, Infallible>();
        // Pulling the second source completes the first: the pull goes on into the room it makes.
        let sources = {
            let channels = channels.clone();
            (0..).map(move |index| {
                if index == 1 {
                    channels.on_termination(0, Termination::Completed);
                }
                source.clone()
            })
        };
        let (checker, observer) = Checker::new();

        let _subscription = Merge::new_from_iter(sources)
            .max_concurrency(TWO)
            .subscribe(observer);
        assert_eq!(channels.len(), 3);
        assert_eq!(channels.state(0), ChannelState::Completed);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(channels.state(2), ChannelState::Subscribed);
        assert_eq!(checker.state(), State::Active);
    }

    #[test]
    fn test_error() {
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
        let (sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, &str>();
        let (_sender_3, observable_3, channel_checker_3) = test_channel::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        let _subscription = Merge::new((observable_1, observable_2, observable_3))
            .max_concurrency(TWO)
            .subscribe(observer);
        assert!(sender_1.on_next(1).is_continue());
        sender_2.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [1]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
        // The third, waiting for room, is never subscribed.
        assert_eq!(channel_checker_3.state(), ChannelState::Initialized);
    }

    #[test]
    fn test_complete_on_sub() {
        let (sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // The first source completes as it is subscribed, which makes room for the second at
        // once; so does the last, which completes the stream with it.
        let _subscription = Merge::new((Just::new(1), observable_2, Just::new(3)))
            .max_concurrency(ONE)
            .subscribe(observer);
        assert_eq!(checker.values(), [1]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [1, 3]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_stop_during_iterator_next_skips_subscription() {
        let (channels, source) = test_channels::<'_, i32, Infallible>();
        // There is room for the second source, but pulling it ends the stream. The iterator owns
        // its handles: the observers the channels keep hold it.
        let sources = {
            let channels = channels.clone();
            (0..).map(move |index| {
                if index == 1 {
                    assert!(channels.on_next(0, 42).is_stop());
                }
                source.clone()
            })
        };
        let (checker, observer) = Checker::new();
        let _subscription = Merge::new_from_iter(sources)
            .max_concurrency(TWO)
            .take(1)
            .subscribe(observer);
        assert_eq!(checker.values(), [42]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.len(), 1);
    }

    #[test]
    fn test_pull_on_completing_thread() {
        block_on(|_| async move {
            let thread_1 = ThreadCheckerExecutor::new("thread_1");
            let pulled_on = Arc::new(Mutex::new(Vec::new()));
            let sources = {
                let (pulled_on, scheduler) = (pulled_on.clone(), thread_1.scheduler());
                (0..3).map(move |_| {
                    pulled_on.with_mut(|pulled_on| pulled_on.push(get_thread_name()));
                    Empty.subscribe_on(scheduler.clone())
                })
            };
            let (checker, observer) = Checker::new();

            let _subscription = Merge::new_from_iter(sources)
                .max_concurrency(ONE)
                .subscribe(observer);
            assert_eq!(pulled_on.clone_value(), [None]);

            // Each source completes on thread_1, where the next one is pulled.
            thread_1.run_until_stalled().await;
            assert_eq!(
                pulled_on.clone_value(),
                [None, Some("thread_1"), Some("thread_1")]
            );
            assert_eq!(checker.state(), State::Completed::<Infallible>);
        });
    }

    #[test]
    fn test_error_from_iter() {
        let (channels, source) = test_channels::<'_, i32, &str>();
        let pulled = Arc::new(AtomicUsize::new(0));
        let sources = {
            let pulled = pulled.clone();
            (0..).map(move |_| {
                pulled.fetch_add(1, Ordering::SeqCst);
                source.clone()
            })
        };
        let (checker, observer) = Checker::new();

        let _subscription = Merge::new_from_iter(sources)
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        channels.on_termination(0, Termination::Error("error"));
        assert_eq!(checker.values(), [1]);
        assert_eq!(checker.state(), State::Error("error"));
        // Nothing is pulled once the stream is over.
        assert_eq!(pulled.load(Ordering::SeqCst), 1);
        assert_eq!(channels.len(), 1);
    }

    #[test]
    fn test_unsub_on_next_by_take() {
        let pulled = Arc::new(AtomicUsize::new(0));
        let sources = {
            let pulled = pulled.clone();
            (0..).map(move |value| {
                pulled.fetch_add(1, Ordering::SeqCst);
                Just::new(value)
            })
        };
        let (checker, observer) = Checker::new();
        let _subscription = Merge::new_from_iter(sources)
            .max_concurrency(TWO)
            .take(3)
            .subscribe(observer);
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Completed::<Infallible>);
        assert_eq!(pulled.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_release_on_completion() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let observers = Arc::new(Mutex::new(Vec::new()));
        let (checker, observer) = Checker::new();

        let subscription = Merge::new_from_iter((1..).map(logged_source!(log, observers)))
            .max_concurrency(ONE)
            .subscribe(observer);
        assert_eq!(log.clone_value(), [1]);

        // The subscription of a source that completed is disposed at once, then the next one is
        // pulled: a list that never ends does not hold every subscription it made.
        let first = observers.with_mut(|observers| observers[0].take().unwrap());
        first.on_termination(Termination::Completed);
        assert_eq!(log.clone_value(), [1, -1, 2]);

        subscription.dispose();
        assert_eq!(log.clone_value(), [1, -1, 2, -2]);
        assert_eq!(checker.state(), State::Active);
    }

    #[test]
    fn test_disposal_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let observers = Arc::new(Mutex::new(Vec::new()));
        let source = logged_source!(log, observers);

        let subscription = Merge::new((source(1), source(2), source(3)))
            .max_concurrency(TWO)
            .subscribe_with_callback(|_| {}, |_| {});
        assert_eq!(log.clone_value(), [1, 2]);

        // The last subscribed first; the third, never subscribed, is dropped.
        subscription.dispose();
        assert_eq!(log.clone_value(), [1, 2, -2, -1]);
    }

    #[test]
    fn test_error_on_completed_source_unsub() {
        for from_iter in [false, true] {
            let (sender, first, _) = test_channel::<'_, i32, &str>();
            let (_, last, last_checker) = test_channel();
            let completing =
                Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, &str>| {
                    observer.on_termination(Termination::Completed);
                    DisposeOnDrop::new(CallbackDisposal::new(move || {
                        sender.on_termination(Termination::Error("error"));
                    }))
                });
            let (checker, observer) = Checker::new();
            let pulled = Arc::new(AtomicUsize::new(0));

            if from_iter {
                let pulled_next = pulled.clone();
                Merge::new_from_iter(
                    [
                        first.into_send_boxed(),
                        completing.into_send_boxed(),
                        last.into_send_boxed(),
                    ]
                    .into_iter()
                    .inspect(move |_| {
                        pulled_next.fetch_add(1, Ordering::SeqCst);
                    }),
                )
                .max_concurrency(TWO)
                .subscribe(observer)
                .dispose();
            } else {
                Merge::new((first, completing, last))
                    .max_concurrency(TWO)
                    .subscribe(observer)
                    .dispose();
            }
            assert_eq!(checker.state(), State::Error("error"));
            assert_eq!(
                last_checker.state(),
                ChannelState::Initialized,
                "from_iter = {from_iter}"
            );
            if from_iter {
                assert_eq!(pulled.load(Ordering::SeqCst), 2);
            }
        }
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_unsubscribe_on_panic_while_releasing() {
        use crate::tests_utils::panic::expect_panic_on_drop;

        let (sender, first, _) = test_channel::<'_, i32, Infallible>();
        let (_, second, second_checker) = test_channel();
        let (_, last, last_checker) = test_channel();
        let (checker, observer) = Checker::new();
        // Keep the handle alive across the unwind: cleanup must come from the operator itself.
        let mut subscription = None;

        expect_panic_on_drop(|panic_on_drop| {
            let first = first.do_after_disposal(move || drop(panic_on_drop));
            subscription = Some(
                Merge::new((first, second, last))
                    .max_concurrency(TWO)
                    .subscribe(observer),
            );
            sender.on_termination(Termination::Completed);
        });
        assert_eq!(second_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(last_checker.state(), ChannelState::Initialized);
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_error_on_completed_source_unsub_after_sub() {
        let (first_sender, first, _) = test_channel::<'_, i32, &str>();
        let (error_sender, second, _) = test_channel();
        let (_, last, last_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let first = first.do_after_disposal(move || {
            error_sender.on_termination(Termination::Error("error"));
        });
        let _subscription = Merge::new((first, second, last))
            .max_concurrency(TWO)
            .subscribe(observer);

        first_sender.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(last_checker.state(), ChannelState::Initialized);
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_unsubscribe_on_panic_while_pulling() {
        use crate::tests_utils::panic::expect_panic_on_drop;

        let log = Arc::new(Mutex::new(Vec::new()));
        let observers = Arc::new(Mutex::new(Vec::new()));
        let source = logged_source!(log, observers);
        let (checker, observer) = Checker::new();
        // Keep the handle alive across the unwind: cleanup must come from the operator itself.
        let mut subscription = None;

        expect_panic_on_drop(|panic_on_drop| {
            let mut panic_on_drop = Some(panic_on_drop);
            let sources = (1..).map(move |index| {
                if index == 3 {
                    // The iterator fails.
                    drop(panic_on_drop.take());
                }
                source(index)
            });
            subscription = Some(
                Merge::new_from_iter(sources)
                    .max_concurrency(TWO)
                    .subscribe(observer),
            );
            assert_eq!(log.clone_value(), [1, 2]);

            // The completion pulls the next source, which panics: the stream is given up, and the
            // source still running is disposed, rather than left waiting for a pull that never
            // comes.
            let first = observers.with_mut(|observers| observers[0].take().unwrap());
            first.on_termination(Termination::Completed);
        });
        assert_eq!(log.clone_value(), [1, 2, -1, -2]);
        assert!(checker.values().is_empty());
    }

    #[test]
    fn test_race_condition() {
        const ROUNDS: usize = 20;
        const SOURCES: usize = 40;
        const THREADS: usize = 4;
        const TIMEOUT: Duration = Duration::from_secs(5);

        for _ in 0..ROUNDS {
            let (channels, source) = test_channels::<'_, usize, Infallible>();
            let active = Arc::new(AtomicUsize::new(0));
            let max_active = Arc::new(AtomicUsize::new(0));
            let (done_sender, done) = mpsc::channel();
            let (checker, observer) = Checker::new();

            let sources: Vec<_> = (0..SOURCES)
                .map(|_| {
                    let (active_up, max_active, active_down) =
                        (active.clone(), max_active.clone(), active.clone());
                    source
                        .clone()
                        .do_before_subscription(move || {
                            let now = active_up.fetch_add(1, Ordering::SeqCst) + 1;
                            max_active.fetch_max(now, Ordering::SeqCst);
                        })
                        .do_before_termination(move |_| {
                            active_down.fetch_sub(1, Ordering::SeqCst);
                        })
                })
                .collect();
            // Completions on several threads pull the next sources there.
            let _subscription = Merge::new_from_iter(sources)
                .max_concurrency(THREE)
                .do_after_termination(move |_| done_sender.send(()).unwrap())
                .subscribe(observer);

            thread::scope(|scope| {
                for thread_index in 0..THREADS {
                    let channels = &channels;
                    scope.spawn(move || {
                        for index in (thread_index..SOURCES).step_by(THREADS) {
                            let deadline = Instant::now() + TIMEOUT;
                            while channels.len() <= index
                                || channels.state(index) != ChannelState::Subscribed
                            {
                                assert!(Instant::now() < deadline, "source {index} never pulled");
                                thread::yield_now();
                            }
                            assert!(channels.on_next(index, index).is_continue());
                            channels.on_termination(index, Termination::Completed);
                        }
                    });
                }
            });

            done.recv_timeout(TIMEOUT).unwrap();
            assert!(max_active.load(Ordering::SeqCst) <= 3);
            let mut values = checker.values();
            values.sort_unstable();
            assert_eq!(values, (0..SOURCES).collect::<Vec<_>>());
            assert_eq!(checker.state(), State::Completed);
        }
    }
}
