mod tests_utils;

use crate::tests_utils::{
    DURATION_30_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
    test_runtime::block_on,
    test_struct::TestStruct,
};
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::Subscription,
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, utility::timestamp::Timestamp},
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, fmt::Debug, ops::Sub, time::Instant};

fn check_values<T: PartialEq + Debug>(values: Vec<(T, Instant)>, expected: Vec<(T, Instant)>) {
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
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(333);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_termination(Termination::<Infallible>::Completed);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(333);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_termination(Termination::Error("error"));
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(333);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        subscription.dispose();
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_ref() {
    block_on(|runtime| async move {
        let value_1 = 111;
        let value_2 = 222;
        let value_3 = 333;
        let error = "error";
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(&value_1);
        check_values(checker.values(), vec![(&value_1, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(&value_2);
        check_values(checker.values(), vec![(&value_1, start), (&value_2, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(&value_3);
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

        runtime.sleep(DURATION_30_MS).await;
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
    });
}

#[test]
fn test_mut_ref() {
    block_on(|runtime| async move {
        let mut value_1 = 111;
        let mut value_2 = 222;
        let mut value_3 = 333;
        let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.map(|v| (*v.0 * 2, v.1)).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(&mut value_1);
        check_values(checker.values(), vec![(222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(&mut value_2);
        check_values(checker.values(), vec![(222, start), (444, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(&mut value_3);
        check_values(
            checker.values(),
            vec![(222, start), (444, start), (666, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_termination(Termination::<Infallible>::Completed);
        check_values(
            checker.values(),
            vec![(222, start), (444, start), (666, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        let sender = runtime
            .spawn(async move {
                sender.on_next(333);
                sender
            })
            .await
            .unwrap();
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        runtime
            .spawn(async move {
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone().timestamp();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(111);
        check_values(checker_1.values(), vec![(111, start)]);
        assert_eq!(checker_1.state(), State::Active);
        check_values(checker_2.values(), vec![(111, start)]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(222);
        check_values(checker_1.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker_1.state(), State::Active);
        check_values(checker_2.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        subject.on_next(333);
        check_values(
            checker_1.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker_1.state(), State::Active);
        check_values(
            checker_2.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        subject.on_termination(Termination::<Infallible>::Completed);
        check_values(
            checker_1.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker_1.state(), State::Completed);
        check_values(
            checker_2.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timestamp().take(1);
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(111);
        check_values(checker.values(), vec![(111, start + DURATION_30_MS)]);
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
        let observable = observable.timestamp().timestamp();
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
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

        sender.on_next(111);
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(333);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_termination(Termination::<Infallible>::Completed);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
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
        let observable = Timestamp::new(observable);
        runtime.sleep(DURATION_30_MS).await;

        let start = Instant::now();
        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        check_values(checker.values(), vec![(111, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(222);
        check_values(checker.values(), vec![(111, start), (222, start)]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_next(333);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        sender.on_termination(Termination::<Infallible>::Completed);
        check_values(
            checker.values(),
            vec![(111, start), (222, start), (333, start + DURATION_30_MS)],
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
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

        let observable = observable.timestamp();

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
            Subscription::default()
        });
        let observable = observable.timestamp();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next((Some(&life_marker_2), Instant::now()));
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.timestamp();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    let observable = observable.timestamp();
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    observable.timestamp();
}
