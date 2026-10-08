mod tests_utils;

use crate::tests_utils::DURATION_1_MS;
use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::never::Never, filtering::debounce::Debounce},
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    for _ in 0..10 {
        assert!(sender.on_next(0).is_continue());
        time.advance_by(DURATION_100_MS - DURATION_1_MS);
    }
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    for _ in 0..10 {
        assert!(sender.on_next(0).is_continue());
        time.advance_by(DURATION_100_MS - DURATION_1_MS);
    }
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();
    let observable_3 = observable_2.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert!(channels.on_next(2, 111).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped); // This assert is ok in multi-threaded because the scheduler is finished.
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    assert!(channels.on_next(2, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    assert!(channels.on_next(1, 333).is_continue());
    assert!(channels.on_next(2, 333).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(checker_3.values(), [111, 222, 333]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);

    channels.on_termination(2, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(checker_3.values(), [111, 222, 333]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(channels.state(2), ChannelState::Error("error"));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(&111).is_continue());
                sender
            })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .debounce(DURATION_100_MS, scheduler.clone())
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    for _ in 0..10 {
        assert!(sender.on_next(0).is_continue());
        time.advance_by(DURATION_100_MS - DURATION_1_MS);
    }
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

/// Downstream ends its own stream on the timer's thread: the source is disposed there and then,
/// not left subscribed until it sends again.
#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = observable
        .debounce(DURATION_100_MS, scheduler.clone())
        .subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .debounce(DURATION_100_MS, scheduler.clone())
        .debounce(DURATION_100_MS + DURATION_30_MS * 2, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS * 2 + DURATION_30_MS * 2 - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(0).is_continue());
    time.advance_by(DURATION_100_MS - DURATION_30_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(0).is_continue());
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    time.advance_by(DURATION_100_MS + DURATION_30_MS * 3);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    time.advance_by(DURATION_100_MS * 2 + DURATION_30_MS * 3);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Debounce::new(observable, DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_complete_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    subscription.dispose();
    // The timer task still holds the context, but the disposal releases the observer itself.
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

/// The source goes away without a termination while a value is pending: unlike a completion, which
/// flushes it at once, the source only fell silent, so the value is emitted when the quiet period
/// ends, and the observer is then dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
}

/// Disposing after the source went away still cancels the value it left pending. The timer task
/// alone holds the context then, so the observer is released when the runtime drops the cancelled
/// task, as it is when the subscription is disposed after the source terminated.
#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    subscription.dispose();
    time.advance_by(DURATION_10_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
}

#[test]
fn test_unsub_after_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsub_after_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let subject = BehaviorSubject::<_, Infallible, _>::shared(111);
    let (checker, observer) = Checker::new();

    let observable = subject.clone().debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    let observable = Empty.debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").debounce(DURATION_100_MS, scheduler.clone());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_error_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.debounce(DURATION_100_MS, scheduler.clone());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Never.debounce(DURATION_100_MS, scheduler.clone());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Never.debounce(DURATION_100_MS, scheduler.clone());

    observable.filter(|_| true);
}
