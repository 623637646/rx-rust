mod tests_utils;

use crate::tests_utils::DURATION_3_MS;
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
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_time_last_empty() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_time_last_not_empty() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_no_delay() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            None,
        );

        let _subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_time_small_delay() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_10_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_count_last_empty() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(3).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(666).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_count_last_not_empty() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(3).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_time_and_count() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert!(sender.on_next(666).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(777).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error_time_and_count() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert!(sender.on_next(666).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(777).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|scheduler| async move {
        let (channels, observable) = test_channels();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 333).is_continue());
        assert!(channels.on_next(1, 333).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 444).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 555).is_continue());
        assert!(channels.on_next(1, 666).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 777).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        channels.on_termination(1, Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert_eq!(checker_2.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Completed);
    });
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(333).is_continue());
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(444).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(555).is_continue());
                assert!(sender.on_next(666).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(777).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;

        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|scheduler| async move {
        let (channels, observable) = test_channels();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 333).is_continue());
        assert!(channels.on_next(1, 333).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 444).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 555).is_continue());
        assert!(channels.on_next(1, 666).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(1, 777).is_continue());
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        channels.on_termination(1, Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert_eq!(checker_2.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|scheduler| async move {
        let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .buffer_with_time_or_count(
                NonZeroUsize::new(100).unwrap(),
                DURATION_100_MS,
                scheduler.clone(),
                Some(DURATION_100_MS),
            )
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                DURATION_100_MS,
                scheduler.clone(),
                Some(DURATION_100_MS),
            )
            .buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                DURATION_100_MS + DURATION_30_MS,
                scheduler.clone(),
                Some(DURATION_100_MS + DURATION_30_MS),
            );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS * 2).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert!(sender.on_next(444).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert!(sender.on_next(555).is_continue());
        assert!(sender.on_next(666).is_continue());
        assert!(sender.on_next(666).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]],
                vec![vec![], vec![555, 555]]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]],
                vec![vec![], vec![555, 555]]
            ]
        );
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = BufferWithTimeOrCount::new(
            observable,
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(333).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(444).is_continue());
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(555).is_continue());
        assert!(sender.on_next(666).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(777).is_continue());
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|scheduler| async move {
        let (sender, source, _) = test_channel();
        let source = source.start_with([111]);
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = source;
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            None,
        );

        let _subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Active);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Empty.buffer_with_time_or_count(
            NonZeroUsize::new(1).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_on_sub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Throw::new("error").buffer_with_time_or_count(
            NonZeroUsize::new(1).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source emits from inside its own disposal, so the value arrives while downstream is
        // unsubscribing. It must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_stop());
                }))
            });

        // Custom operations
        // Delayed: with no delay the first tick fires at once, on a worker thread, and would race
        // the disposal below with an empty buffer.
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source completes from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The termination must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        // Custom operations
        // Delayed: with no delay the first tick fires at once, on a worker thread, and would race
        // the disposal below with an empty buffer.
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_error_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source fails from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The error must be dropped instead of reaching the observer.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        });

        // Custom operations
        // Delayed: with no delay the first tick fires at once, on a worker thread, and would race
        // the disposal below with an empty buffer.
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_lifetime_sub() {
    block_on(|scheduler| async move {
        // OK
        let life_marker = TestStruct;
        let _subscription;

        // Error
        // let _subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::shared_boxed(move |mut observer| {
                assert!(observer.on_next(111).is_continue());
                Subscription::new(CallbackDisposal::new(move || {
                    life_marker.consume_ref();
                }))
            });

            let observable = observable.buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                DURATION_100_MS,
                scheduler.clone(),
                Some(DURATION_100_MS),
            );

            let (_, observer) = Checker::<_, ()>::new();
            _subscription = observable.subscribe(observer);
        }
    });
}

#[test]
fn test_clone() {
    block_on(|scheduler| async move {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(TestStruct).is_continue());
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|scheduler| async move {
        // Custom operations
        let (_, observable, _) = test_channel::<'_, i32, String>();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|scheduler| async move {
        // Custom operations
        let (_, observable, _) = test_channel::<'_, i32, String>();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
            Some(DURATION_100_MS),
        );

        observable.filter(|_| true);
    });
}
