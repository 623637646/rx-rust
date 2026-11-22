mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel, test_channels};
use crate::tests_utils::test_scheduler::block_on;
use crate::tests_utils::{DURATION_1_MS, DURATION_10_MS, DURATION_100_MS};
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::operators::creating::create::Create;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::just::Just, utility::delay_subscription::DelaySubscription},
};
use std::convert::Infallible;
use std::time::Duration;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.delay_subscription(DURATION_100_MS, time.scheduler());

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // Once subscribed, the items pass through at once.
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    subscription.dispose();
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .delay_subscription(DURATION_100_MS, time.scheduler())
        .subscribe(observer);
    time.advance_by(DURATION_100_MS);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_completed_zero_initial_delay() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .delay_subscription(Duration::ZERO, time.scheduler())
        .subscribe(observer);
    // The subscription still goes through the scheduler: it runs once the scheduler does.
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
    time.advance_by(Duration::ZERO);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_hot_source_misses_earlier_values() {
    let time = VirtualTime::new();
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let _subscription = subject
        .clone()
        .delay_subscription(DURATION_100_MS, time.scheduler())
        .subscribe(observer);

    assert!(subject.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(subject.on_next(222).is_continue());
    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable.delay_subscription(DURATION_100_MS, time.scheduler());

    // Disposed before the delay is over: the source is never subscribed.
    let subscription_1 = observable.clone().subscribe(observer_1);
    subscription_1.dispose();
    assert_eq!(checker_1.state(), State::Dropped);
    let subscription_2 = observable.clone().subscribe(observer_2);
    let _subscription_3 = observable.subscribe(observer_3);
    time.advance_by(DURATION_100_MS);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_3.values(), [111]);

    // Disposed once subscribed: the source is disposed.
    subscription_2.dispose();
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_3.state(), State::Active);

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.delay_subscription(DURATION_10_MS, scheduler.clone());

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        scheduler.sleep(DURATION_10_MS * 3).await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let time = VirtualTime::new();
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.delay_subscription(DURATION_100_MS, time.scheduler());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    time.advance_by(DURATION_100_MS);

    for index in 0..2 {
        assert!(channels.on_next(index, 111).is_continue());
        channels.on_termination(index, Termination::Error("error"));
    }
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .delay_subscription(DURATION_100_MS, time.scheduler())
        .take(1);

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS);

    assert!(sender.on_next(111).is_break());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .delay_subscription(DURATION_10_MS, time.scheduler())
        .map(|value| value * 2)
        .delay_subscription(DURATION_100_MS, time.scheduler());

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
    time.advance_by(DURATION_10_MS - DURATION_1_MS);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
    time.advance_by(DURATION_1_MS);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = DelaySubscription::new(observable, DURATION_100_MS, time.scheduler());

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::new();
    let observable = Just::new(111).delay_subscription(DURATION_100_MS, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::<i32, _>::new();
    let observable = Create::shared_boxed(|observer| {
        observer.on_termination(Termination::Error("error"));
        DisposeOnDrop::default()
    })
    .delay_subscription(DURATION_100_MS, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let observable = Create::shared_boxed(|_observer| DisposeOnDrop::default())
        .delay_subscription(DURATION_100_MS, time.scheduler());

    // The source drops the observer as it is subscribed, which the delay does not hold back.
    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_next_on_unsub() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::<_, Infallible>::new();
    let observable = Create::shared_boxed(|mut observer| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            assert!(observer.on_next(111).is_continue());
        }))
    })
    .delay_subscription(DURATION_100_MS, time.scheduler());

    let subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS);
    // Nothing stands between the source and the observer once subscribed: an item sent from the
    // source's own disposal is delivered.
    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.delay_subscription(DURATION_100_MS, time.scheduler());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let observable = Just::new(1).delay_subscription(DURATION_100_MS, time.scheduler());

    let observable = observable.delay_subscription(DURATION_100_MS, time.scheduler());
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let observable = Just::new(1).delay_subscription(DURATION_100_MS, time.scheduler());

    observable.delay_subscription(DURATION_100_MS, time.scheduler());
}
