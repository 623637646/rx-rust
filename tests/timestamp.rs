mod tests_utils;

use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::{
    DURATION_30_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::operators::creating::create::Create;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::utility::timestamp::Timestamp,
};
use std::{convert::Infallible, fmt::Debug, ops::Sub, time::Instant};

/// On the virtual clock the times are exact.
fn check_values<T: PartialEq + Debug>(values: Vec<(T, Instant)>, expected: Vec<(T, Instant)>) {
    assert_eq!(values, expected);
}

/// On a real clock (`test_async`) the times are only close.
fn check_values_within<T: PartialEq + Debug>(
    values: Vec<(T, Instant)>,
    expected: Vec<(T, Instant)>,
) {
    assert_eq!(values.len(), expected.len());
    for i in 0..values.len() {
        let value = &values[i];
        let expected = &expected[i];
        assert_eq!(value.0, expected.0);
        let diff = value.1.max(expected.1).sub(value.1.min(expected.1));
        assert!(diff < DURATION_30_MS, "{value:?} != {expected:?}",);
    }
}

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    check_values(checker.values(), vec![(111, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    check_values(checker.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(333).is_continue());
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::<Infallible>::Completed);
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    check_values(checker.values(), vec![(111, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    check_values(checker.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(333).is_continue());
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::Error("error"));
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    check_values(checker.values(), vec![(111, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    check_values(checker.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(333).is_continue());
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    subscription.dispose();
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = "error";
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    check_values(checker.values(), vec![(&value_1, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    check_values(checker.values(), vec![(&value_1, start), (&value_2, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(&value_3).is_continue());
    check_values(
        checker.values(),
        vec![
            (&value_1, start),
            (&value_2, start),
            (&value_3, start + DURATION_30_MS),
        ],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::Error(&error));
    check_values(
        checker.values(),
        vec![
            (&value_1, start),
            (&value_2, start),
            (&value_3, start + DURATION_30_MS),
        ],
    );
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;
    let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.map(|v| (*v.0 * 2, v.1)).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&mut value_1).is_continue());
    check_values(checker.values(), vec![(222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&mut value_2).is_continue());
    check_values(checker.values(), vec![(222, start), (444, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(&mut value_3).is_continue());
    check_values(
        checker.values(),
        vec![(222, start), (444, start), (666, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::<Infallible>::Completed);
    check_values(
        checker.values(),
        vec![(222, start), (444, start), (666, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.timestamp(scheduler.clone());
        scheduler.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        check_values_within(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        check_values_within(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        let sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await;
        check_values_within(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_30_MS).await;
        scheduler
            .spawn(async move {
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await;
        check_values_within(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.timestamp(scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();
    time.advance_by(DURATION_30_MS);

    let start = time.now();
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
    check_values(checker_1.values(), vec![(111, start)]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    check_values(checker_2.values(), vec![(111, start)]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 222).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    check_values(checker_1.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    check_values(checker_2.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(channels.on_next(0, 333).is_continue());
    assert!(channels.on_next(1, 333).is_continue());
    check_values(
        checker_1.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    check_values(
        checker_2.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    channels.on_termination(0, Termination::<Infallible>::Completed);
    channels.on_termination(1, Termination::<Infallible>::Completed);
    check_values(
        checker_1.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    check_values(
        checker_2.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.timestamp(scheduler.clone()).take(1);
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(111).is_stop());
    check_values(checker.values(), vec![(111, start + DURATION_30_MS)]);
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
        .timestamp(scheduler.clone())
        .timestamp(scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    fn check_values<T: PartialEq + Debug>(
        values: Vec<((T, Instant), Instant)>,
        expected: Vec<(T, Instant)>,
    ) {
        assert_eq!(values.len(), expected.len());
        for i in 0..values.len() {
            let value = &values[i];
            let expected = &expected[i];
            assert_eq!(value.0.0, expected.0);
            let diff = value.0.1.max(expected.1).sub(value.0.1.min(expected.1));
            assert!(diff < DURATION_30_MS, "{value:?} != {expected:?}",);
            let diff = value.1.max(expected.1).sub(value.1.min(expected.1));
            assert!(diff < DURATION_30_MS, "{value:?} != {expected:?}",);
        }
    }

    assert!(sender.on_next(111).is_continue());
    check_values(checker.values(), vec![(111, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    check_values(checker.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(333).is_continue());
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::<Infallible>::Completed);
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Timestamp::new(observable, scheduler.clone());
    time.advance_by(DURATION_30_MS);

    let start = time.now();
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    check_values(checker.values(), vec![(111, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    check_values(checker.values(), vec![(111, start), (222, start)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    assert!(sender.on_next(333).is_continue());
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_30_MS);
    sender.on_termination(Termination::<Infallible>::Completed);
    check_values(
        checker.values(),
        vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_lifetime_sub() {
    let time = VirtualTime::new();
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.timestamp(time.scheduler());

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    let time = VirtualTime::new();
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.timestamp(time.scheduler());

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(
            observer
                .on_next((Some(&life_marker_2), Instant::now()))
                .is_continue()
        );
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.timestamp(time.scheduler());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    let observable = observable.timestamp(time.scheduler());
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    observable.timestamp(time.scheduler());
}
