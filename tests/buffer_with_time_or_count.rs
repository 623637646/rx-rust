mod tests_utils;

use crate::tests_utils::DURATION_1_MS;
use crate::tests_utils::DURATION_1_YEAR;
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
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
};
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_time_last_empty() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_time_last_not_empty() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
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
}

/// A time span too long for an `Instant` to represent never ends: buffers are emitted by count and
/// by the completion only, and the task waits until the subscription ends.
#[test]
fn test_time_span_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        Duration::MAX,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert!(checker.values().is_empty());
    assert_eq!(time.pending_tasks(), 1);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(checker.values(), [vec![111, 222]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(time.pending_tasks(), 1);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222], vec![333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// The same as `buffer_with_time`: a timer that never comes due still holds the observer, so the
/// source dropping its observer releases nothing, the disposal does (decision 0004).
#[test]
fn test_time_span_too_long_abandon_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        Duration::MAX,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    time.advance_by(DURATION_1_YEAR);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 1);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

/// A full bundle restarts the timer: the first timed bundle comes `time_span` after it, not
/// `time_span` after the subscription.
#[test]
fn test_full_bundle_restarts_timer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS - DURATION_30_MS);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222]]);

    time.advance_by(DURATION_30_MS);
    assert_eq!(checker.values(), [vec![111, 222]]);

    time.advance_by(DURATION_100_MS - DURATION_30_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111, 222]]);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111, 222], vec![333]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

#[test]
fn test_completed_count_last_empty() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(3).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

#[test]
fn test_completed_count_last_not_empty() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(3).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

#[test]
fn test_completed_time_and_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
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
}

#[test]
fn test_error_time_and_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
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
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
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
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        // Half the durations of the other tests: this one runs on real time, on every scheduler.
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS / 2,
            scheduler.clone(),
        );

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS / 2).await;
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

        scheduler.sleep(DURATION_30_MS / 2).await;
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

        scheduler.sleep(DURATION_30_MS / 2).await;
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

        scheduler.sleep(DURATION_30_MS / 2).await;
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

        scheduler.sleep(DURATION_100_MS / 2).await;
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

        scheduler.sleep(DURATION_30_MS / 2).await;
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

        scheduler.sleep(DURATION_100_MS / 2).await;
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

        scheduler.sleep(DURATION_100_MS / 2).await;
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
        scheduler.sleep(DURATION_10_MS / 2).await;

        scheduler.sleep(DURATION_10_MS / 2).await;
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
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
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
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS + DURATION_30_MS,
            scheduler.clone(),
        );

    // The batches of 111, 222 and 333 are all cut by count at 0 ms, before either timer
    // fires. The 444s are cut by count at 60 ms. The first ticks at 100 and 130 ms resync the
    // inner timer to 160 ms and the outer one to 130 ms, which emits [[444, 444]]; the inner
    // tick at 160 ms hands an empty buffer to the outer one, whose next tick is at 260 ms. The
    // checks at 0, 60 and 220 ms sit at least 40 ms away from every tick.
    let _subscription = observable.subscribe(observer);
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

    time.advance_by(DURATION_30_MS * 2);
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

    time.advance_by(DURATION_100_MS + DURATION_30_MS * 2);
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
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = BufferWithTimeOrCount::new(
        observable,
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert_eq!(checker.values(), [vec![111, 111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_30_MS);
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

    time.advance_by(DURATION_100_MS);
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

    time.advance_by(DURATION_100_MS);
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
}

#[test]
fn test_complete_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

#[test]
fn test_error_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    subscription.dispose();
    time.advance_by(DURATION_10_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_after_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

#[test]
fn test_unsub_after_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
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
}

/// The source goes away without a termination: the buffers are cut by time, not by the source, so
/// the open one is still emitted at its tick and the ticks go on, with empty buffers, until the
/// subscription is disposed.
/// See decision 0004: a source that drops its observer without a termination only stops sending;
/// what the operator has already accepted runs its course.
#[test]
fn test_abandon_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![111], vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    subscription.dispose();
    time.advance_by(DURATION_10_MS);
    assert_eq!(checker.values(), [vec![111], vec![]]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [vec![111], vec![]]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
}

/// Disposing after the source went away cancels the ticks: the open buffer is never emitted.
#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(100).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

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
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (checker, observer) = Checker::new();

    let observable = source;
    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    let observable = Empty.buffer_with_time_or_count(
        NonZeroUsize::new(1).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").buffer_with_time_or_count(
        NonZeroUsize::new(1).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
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
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

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
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

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
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

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
        DisposeOnDrop::default()
    });
    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.buffer_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    observable.filter(|_| true);
}
