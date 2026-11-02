mod tests_utils;

use crate::tests_utils::DURATION_1_MS;
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
use rx_rust::operators::creating::never::Never;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::operators::creating::timer::Timer;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Flow, Observer, Termination},
    operators::combining::join::Join,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Subscribed);
    assert_eq!(right_checker.state(), ChannelState::Subscribed);

    // 1 opens a window, and finds nothing to meet yet.
    assert!(left.on_next(1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_durations.len(), 1);
    assert_eq!(left_durations.state(0), ChannelState::Subscribed);

    assert!(right.on_next("a").is_continue());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_durations.len(), 1);
    assert_eq!(right_durations.state(0), ChannelState::Subscribed);

    assert!(left.on_next(2).is_continue());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_durations.len(), 2);
    assert_eq!(left_durations.state(1), ChannelState::Subscribed);

    // The duration of "a" emits: its window closes, and the duration is not needed any more.
    assert!(right_durations.on_next(0, ()).is_stop());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_durations.state(0), ChannelState::Unsubscribed);

    // "b" meets 1 and 2, in the order they arrived.
    assert!(right.on_next("b").is_continue());
    assert_eq!(checker.values(), [(1, "a"), (2, "a"), (1, "b"), (2, "b")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_durations.state(1), ChannelState::Subscribed);

    // The duration of 1 completes: its window closes too.
    left_durations.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [(1, "a"), (2, "a"), (1, "b"), (2, "b")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_durations.state(0), ChannelState::Completed);

    assert!(right.on_next("c").is_continue());
    assert_eq!(
        checker.values(),
        [(1, "a"), (2, "a"), (1, "b"), (2, "b"), (2, "c")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_durations.state(2), ChannelState::Subscribed);

    // "b" and "c" are open for left values that will not come: their windows close. 2 is still
    // open for the right values to come.
    left.on_termination(Termination::Completed);
    assert_eq!(
        checker.values(),
        [(1, "a"), (2, "a"), (1, "b"), (2, "b"), (2, "c")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Subscribed);
    assert_eq!(left_durations.state(1), ChannelState::Subscribed);
    assert_eq!(right_durations.state(1), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(2), ChannelState::Unsubscribed);

    // "d" meets 2, and opens no window of its own.
    assert!(right.on_next("d").is_continue());
    assert_eq!(
        checker.values(),
        [(1, "a"), (2, "a"), (1, "b"), (2, "b"), (2, "c"), (2, "d")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_durations.len(), 3);

    // The last left window closes: no pair can come any more.
    assert!(left_durations.on_next(1, ()).is_stop());
    assert_eq!(
        checker.values(),
        [(1, "a"), (2, "a"), (1, "b"), (2, "b"), (2, "c"), (2, "d")]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_durations.state(1), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_from_right() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert_eq!(checker.values(), [(1, "a")]);

    // 1 is open for right values that will not come: its window closes. "a" stays open.
    right.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Subscribed);
    assert_eq!(right_checker.state(), ChannelState::Completed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(0), ChannelState::Subscribed);

    // 2 meets "a", and opens no window of its own.
    assert!(left.on_next(2).is_continue());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_durations.len(), 1);

    right_durations.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
}

/// A completed source whose windows are all closed leaves the other source's values nothing to
/// meet, so the stream completes even though the other source goes on.
#[test]
fn test_completed_without_open_windows() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Empty,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(1).is_continue());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_both_sources() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    left.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    right.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert_eq!(checker.values(), [(1, "a")]);

    left.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_right() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    right.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_from_duration() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    right_durations.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(0), ChannelState::Error("error"));
}

/// A duration whose window was closed by the completion of the other source is disposed: an error
/// it still sends, from another thread say, is dropped.
#[test]
fn test_error_from_closed_window_is_dropped() {
    let (mut left, left_observable, _) = test_channel::<'_, i32, _>();
    let (mut right, right_observable, _) = test_channel();
    let duration_observer = Arc::new(Mutex::new(None));
    let (checker, observer) = Checker::new();

    let duration_observer_cloned = duration_observer.clone();
    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        move |_| {
            let duration_observer = duration_observer_cloned.clone();
            Create::shared_boxed(move |observer: SendBoxedObserver<'_, (), &str>| {
                duration_observer.replace_value(Some(observer));
                DisposeOnDrop::default()
            })
        },
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    // 1 is still open, for the right values to come; "a" waits for left values only.
    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);

    let duration_observer = duration_observer.take_value().unwrap();
    duration_observer.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    right.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsubscribe() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
        |left, right| (*left, *right),
    );

    let subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(&value_1).is_continue());
    assert!(left.on_next(&value_2).is_continue());
    assert!(right.on_next(&value_3).is_continue());
    assert_eq!(
        checker.values(),
        [(&value_1, &value_3), (&value_2, &value_3)]
    );
    assert_eq!(checker.state(), State::Active);

    left.on_termination(Termination::Error(&error));
    assert_eq!(
        checker.values(),
        [(&value_1, &value_3), (&value_2, &value_3)]
    );
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(left_checker.state(), ChannelState::Error(&error));
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
        let (mut right, right_observable, right_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = left_observable.join(
            right_observable,
            |_| Never,
            |_| Never,
            |left, right| (*left, *right),
        );

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let _left = scheduler
            .spawn(async move {
                assert!(left.on_next(111).is_continue());
                left
            })
            .await;
        let _right = scheduler
            .spawn(async move {
                assert!(right.on_next("111").is_continue());
                right
            })
            .await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (left_channels, left_observable) = test_channels();
    let (right_channels, right_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(left_channels.on_next(0, 111).is_continue());
    assert!(right_channels.on_next(0, "111").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert!(checker_2.values().is_empty());

    assert!(left_channels.on_next(1, 222).is_continue());
    assert!(right_channels.on_next(1, "222").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_2.values(), [(222, "222")]);

    left_channels.on_termination(0, Termination::<Infallible>::Completed);
    right_channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.state(), State::Active);

    left_channels.on_termination(1, Termination::Completed);
    right_channels.on_termination(1, Termination::Completed);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .join(
            right_observable,
            |_| Never,
            |_| Never,
            |left, right| (*left, *right),
        )
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(111).is_continue());
    assert!(right.on_next("111").is_stop());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The observer stops on the first of the two pairs a value makes: the second is not made, and
/// the source that sent the value is told to stop.
#[test]
fn test_stop_on_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::stopping_after(1);

    let observable = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(right.on_next("a").is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The observer disposes the subscription on the first of the two pairs a value makes, and
/// answers `Continue`: the second pair is not made.
#[test]
fn test_unsub_on_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();

    let observable = left_observable
        .join(
            right_observable,
            |_| Never,
            |_| Never,
            |left, right| (*left, *right),
        )
        .hook_on_next(move |observer, value| {
            let flow = observer.on_next(value);
            if let Some(subscription) = subscription_cloned.take_value() {
                subscription.dispose();
            }
            flow
        });

    subscription.replace_value(Some(observable.subscribe(observer)));
    assert!(right.on_next("a").is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The result selector disposes the subscription on the first pair of a left value: neither that
/// pair nor the next is sent.
#[test]
fn test_unsub_in_result_selector() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();

    let observable = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        move |left: &i32, right: &&str| {
            if let Some(subscription) = subscription_cloned.take_value() {
                subscription.dispose();
            }
            (*left, *right)
        },
    );

    subscription.replace_value(Some(observable.subscribe(observer)));
    assert!(right.on_next("a").is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(checker.values(), [] as [(i32, &str); 0]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// [`test_unsub_in_result_selector`] for the pairs of a right value.
#[test]
fn test_unsub_in_result_selector_from_right() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();

    let observable = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        move |left: &i32, right: &&str| {
            if let Some(subscription) = subscription_cloned.take_value() {
                subscription.dispose();
            }
            (*left, *right)
        },
    );

    subscription.replace_value(Some(observable.subscribe(observer)));
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("a").is_stop());
    assert_eq!(checker.values(), [] as [(i32, &str); 0]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// Another thread disposes the subscription while a left value is making its pairs, and is held
/// up in the disposal of a right duration: the pair after the one being sent is not made, although
/// the disposal has not returned yet.
#[test]
fn test_unsub_from_other_thread_on_next() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();
    let (dispose_sender, dispose) = mpsc::channel::<()>();
    let (disposing_sender, disposing) = mpsc::channel::<()>();
    let (resume_sender, resume) = mpsc::channel::<()>();
    let blocked_disposal = Arc::new(Mutex::new(Some((disposing_sender, resume))));
    let first_pair = Arc::new(Mutex::new(Some((dispose_sender, disposing))));

    let observable = left_observable
        .join(
            right_observable,
            |_| Never,
            move |_| {
                let blocked_disposal = blocked_disposal.clone();
                Create::shared_boxed(move |_: SendBoxedObserver<'_, (), Infallible>| {
                    let blocked_disposal = blocked_disposal.clone();
                    DisposeOnDrop::new(CallbackDisposal::new(move || {
                        if let Some((disposing, resume)) = blocked_disposal.take_value() {
                            disposing.send(()).unwrap();
                            resume.recv_timeout(TIMEOUT).unwrap();
                        }
                    }))
                })
            },
            |left, right| (*left, *right),
        )
        .hook_on_next(move |observer, value| {
            let flow = observer.on_next(value);
            if let Some((dispose, disposing)) = first_pair.take_value() {
                dispose.send(()).unwrap();
                // The other thread has stopped the context, and is disposing the durations.
                disposing.recv_timeout(TIMEOUT).unwrap();
            }
            flow
        });

    let subscription = observable.subscribe(observer);
    let disposer = thread::spawn(move || {
        dispose.recv_timeout(TIMEOUT).unwrap();
        subscription.dispose();
    });
    assert!(right.on_next("a").is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    resume_sender.send(()).unwrap();
    disposer.join().unwrap();
}

/// A duration that fails while it is subscribed, behind a delivery running on another thread,
/// stops the source of its value: the error is only queued, and the update that stores the
/// duration says the stream is over.
#[test]
fn test_duration_errors_on_sub_while_delivering() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let (mut left, left_observable, _) = test_channel();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();
    let (delivering_sender, delivering) = mpsc::channel();
    let (resume_sender, resume) = mpsc::channel();
    let blocked = Arc::new(Mutex::new(Some((delivering_sender, resume))));

    let observable = left_observable
        .join(
            right_observable,
            |&left: &i32| {
                Create::shared_boxed(move |observer: SendBoxedObserver<'_, (), &str>| {
                    if left == 2 {
                        observer.on_termination(Termination::Error("error"));
                    }
                    DisposeOnDrop::default()
                })
            },
            |_| Never.with_error_type(),
            |left, right| (*left, *right),
        )
        .hook_on_next(move |observer, value| {
            let flow = observer.on_next(value);
            if let Some((delivering, resume)) = blocked.take_value() {
                delivering.send(()).unwrap();
                resume.recv_timeout(TIMEOUT).unwrap();
            }
            flow
        });

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    let deliverer = thread::spawn(move || right.on_next("a").is_stop());
    delivering.recv_timeout(TIMEOUT).unwrap();
    assert!(left.on_next(2).is_stop());
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    resume_sender.send(()).unwrap();
    assert!(deliverer.join().unwrap());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_multiple_operation() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .map(|value| value * 10)
        .join(
            right_observable.filter(|value| value % 2 == 1),
            |_| Never,
            |_| Never,
            |left, right| left + right,
        )
        .map(|value| value * 2);

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next(1).is_continue());
    assert!(right.on_next(2).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next(3).is_continue());
    assert_eq!(checker.values(), [22, 42, 26, 46]);

    left.on_termination(Termination::Completed);
    right.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Join::new(
        left_observable,
        right_observable,
        |_| Never,
        |_| Empty,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("b").is_continue());
    assert_eq!(checker.values(), [(1, "a"), (1, "b"), (2, "b")]);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    right.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

/// A duration that completes during its subscription closes the window at once: its value meets
/// only the values open when it arrives.
#[test]
fn test_duration_completes_on_sub() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Empty,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert!(checker.values().is_empty());

    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(3).is_continue());
    assert_eq!(checker.values(), [(2, "a"), (3, "a"), (3, "b")]);
    assert_eq!(checker.state(), State::Active);
}

/// A duration that fails while it is subscribed ends the stream, and the value that opened it
/// answers its source with a stop: a source emitting while it is subscribed has no disposal yet,
/// so only the flow stops it.
#[test]
fn test_duration_errors_on_sub() {
    let flow = Arc::new(Mutex::new(None));
    let left_observable = Create::shared_boxed({
        let flow = flow.clone();
        move |mut observer: SendBoxedObserver<'_, i32, &str>| {
            let answer = observer.on_next(1);
            flow.replace_value(Some(answer));
            DisposeOnDrop::default()
        }
    });
    let (_, right_observable, right_checker) = test_channel::<'_, &str, _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Throw::new("error"),
        |_| Never.with_error_type(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(flow.clone_value(), Some(Flow::Stop));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Initialized);
}

/// The values are paired by the time their windows overlap, on the scheduler's clock: a window
/// closes exactly when its timer fires.
#[test]
fn test_time_windows() {
    let time = VirtualTime::new();
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let scheduler = time.scheduler();
    let left_scheduler = scheduler.clone();
    let observable = left_observable.join(
        right_observable,
        move |_| Timer::new((), Duration::from_millis(6), left_scheduler.clone()),
        move |_| Timer::new((), Duration::from_millis(2), scheduler.clone()),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next("A").is_continue()); // Open until 6 ms.
    time.advance_by(Duration::from_millis(2));
    assert!(right.on_next(1).is_continue()); // Open until 4 ms.
    assert_eq!(checker.values(), [("A", 1)]);

    time.advance_by(Duration::from_millis(2));
    assert!(left.on_next("B").is_continue()); // 1 closed at 4 ms: B meets nothing.
    assert_eq!(checker.values(), [("A", 1)]);

    time.advance_by(Duration::from_millis(2) - DURATION_1_MS);
    assert!(right.on_next(2).is_continue()); // Just before A closes.
    assert_eq!(checker.values(), [("A", 1), ("A", 2), ("B", 2)]);

    time.advance_by(DURATION_1_MS);
    assert!(right.on_next(3).is_continue()); // A closed at 6 ms, B is open until 10 ms.
    assert_eq!(checker.values(), [("A", 1), ("A", 2), ("B", 2), ("B", 3)]);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    time.advance_by(Duration::from_millis(4) - DURATION_1_MS);
    assert_eq!(checker.state(), State::Active);
    time.advance_by(DURATION_1_MS); // B closes.
    assert_eq!(checker.values(), [("A", 1), ("A", 2), ("B", 2), ("B", 3)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let left_observable = left_observable.start_with([1]);
    let (right, right_observable, _) = test_channel();
    let right_observable = right_observable.start_with(["a"]);
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Active);

    assert!(left.on_next(2).is_continue());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);

    left.on_termination(Termination::Completed);
    right.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::<(i32, i32), Infallible>::new();

    let observable = Empty.with_item_type().join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(right_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_sub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::<(i32, i32), _>::new();

    let observable = Throw::new("error").with_item_type().join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (mut right, right_observable, right_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let subscription = observable.subscribe(observer);
    assert!(right.on_next(1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::<(i32, i32), _>::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.join(
        right_observable,
        |_| Never,
        |_| Never,
        |left, right| (*left, *right),
    );

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::<(i32, i32), _>::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
        |left, right| (*left, *right),
    );

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// its open values still meet those of the other source, until their windows close. Once nothing
/// more can come, the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable.join(
        right_observable,
        move |_| left_duration.clone(),
        |_| Empty,
        |left, right| (*left, *right),
    );

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    left.abandon();
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Abandoned);

    assert!(right.on_next("a").is_continue());
    assert_eq!(checker.values(), [(1, "a")]);
    right.abandon();
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_checker.state(), ChannelState::Abandoned);

    // The window of 1 was the last thing that could make a pair.
    left_durations.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
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
        let left_observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let right_observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(222).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = left_observable.join(
            right_observable,
            |_| Never.with_error_type(),
            |_| Never.with_error_type(),
            |left: &i32, right: &i32| left + right,
        );

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
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
        let left_observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                life_marker_or_1 = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_1.consume_ref();
                }))
            });
        let right_observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                life_marker_or_2 = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_2.consume_ref();
                }))
            });
        let observable = left_observable.join(
            right_observable,
            |_| Never,
            |_| Never,
            |left, right| left + right,
        );

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let left_observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let right_observable =
        Create::shared_boxed(|_: SendBoxedObserver<'_, TestStruct, TestStruct>| {
            DisposeOnDrop::default()
        });
    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
        |_, _| TestStruct,
    );
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, left_observable, _) = test_channel::<'_, i32, String>();
    let (_, right_observable, _) = test_channel::<'_, &str, _>();
    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Empty.with_error_type(),
        |left, right| format!("{left}{right}"),
    );

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, left_observable, _) = test_channel::<'_, i32, String>();
    let (_, right_observable, _) = test_channel::<'_, &str, _>();
    let observable = left_observable.join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Empty.with_error_type(),
        |left, right| format!("{left}{right}"),
    );

    observable.filter(|_| true);
}
