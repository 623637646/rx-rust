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
use crate::tests_utils::window_checker::WindowChecker;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::Flow;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::transforming::window_with_time_or_count::WindowWithTimeOrCount,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_first_tick_after_slow_subscription() {
    for subscription_time in [DURATION_30_MS, DURATION_100_MS + DURATION_30_MS] {
        let time = VirtualTime::new();
        let (mut sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
        let source = source.hook_on_subscription(|source, observer| {
            time.advance_by(subscription_time);
            source.start_with([111]).subscribe(observer)
        });
        let (checker, observer) = WindowChecker::new();
        let _subscription = source
            .window_with_time_or_count(
                NonZeroUsize::new(10).unwrap(),
                DURATION_100_MS,
                time.scheduler(),
            )
            .subscribe(observer);

        if subscription_time < DURATION_100_MS {
            time.advance_by(DURATION_100_MS - subscription_time - DURATION_1_MS);
            assert_eq!(checker.windows(), [(vec![111], State::Active)]);
            time.advance_by(DURATION_1_MS);
        } else {
            time.advance_by(Duration::ZERO);
        }
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Completed), (vec![], State::Active)]
        );
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        let elapsed = subscription_time.max(DURATION_100_MS);
        time.advance_by(DURATION_100_MS * 2 - elapsed - DURATION_1_MS);
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Completed), (vec![222], State::Active)]
        );
        time.advance_by(DURATION_1_MS);
        assert_eq!(
            checker.windows(),
            [
                (vec![111], State::Completed),
                (vec![222], State::Completed),
                (vec![], State::Active),
            ]
        );
    }
}

/// A timer that starts more than one `time_span` late catches up: the rotations it missed follow
/// back to back, with empty windows, and the next one keeps the original rate.
#[test]
fn test_late_timer_catches_up() {
    let time = VirtualTime::new();
    let (_sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
    let source = source.hook_on_subscription(|source, observer| {
        time.advance_by(DURATION_100_MS * 2 + DURATION_30_MS);
        source.start_with([111]).subscribe(observer)
    });
    let (checker, observer) = WindowChecker::new();
    let _subscription = source
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .subscribe(observer);

    time.advance_by(Duration::ZERO);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );

    time.advance_by(DURATION_100_MS - DURATION_30_MS - DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );
    time.advance_by(DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

#[test]
fn test_completed_time() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Active)]
    );
    assert_eq!(checker.state(), State::Active);

    // A window is emitted at every tick, empty or not.
    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );

    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![222, 333], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![222, 333], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_time() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![222], State::Active)]
    );
    assert_eq!(checker.state(), State::Active);

    // The error ends the current window, then the outer observable; the windows before it had
    // completed.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![222], State::Error("error")),
        ]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

/// A time span too long for an `Instant` to represent never ends: short of a full window, the first
/// window stays open until the source terminates.
#[test]
fn test_time_span_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        Duration::MAX,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(time.pending_tasks(), 1);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// A timer that never comes due still holds the observer, as a timer that ticks does: the source
/// dropping its observer releases nothing, the disposal does (decision 0004).
#[test]
fn test_time_span_too_long_abandon_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        Duration::MAX,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 1);

    subscription.dispose();
    assert_eq!(checker.windows(), [(vec![111], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = WindowChecker::new();
    let (checker_2, observer_2) = WindowChecker::new();

    let observable_1 = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(channels.on_next(0, 222).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(
        checker_1.windows(),
        [(vec![111], State::Completed), (vec![222], State::Active)]
    );
    assert_eq!(checker_2.windows(), checker_1.windows());

    // The open window ends without a termination, and the outer observer is released at once.
    subscription_1.dispose();
    assert_eq!(
        checker_1.windows(),
        [(vec![111], State::Completed), (vec![222], State::Dropped)]
    );
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.windows(),
        [(vec![111], State::Completed), (vec![222], State::Active)]
    );
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 333).is_continue());
    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker_1.windows(),
        [(vec![111], State::Completed), (vec![222], State::Dropped)]
    );
    assert_eq!(
        checker_2.windows(),
        [
            (vec![111], State::Completed),
            (vec![222, 333], State::Completed),
            (vec![], State::Active),
        ]
    );

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = WindowChecker::new();

        let observable = observable.window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS / 2,
            scheduler.clone(),
        );

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(checker.windows(), [(vec![], State::Active)]);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.windows(), [(vec![111], State::Active)]);

        scheduler.sleep(DURATION_100_MS / 2 + DURATION_10_MS).await;
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Completed), (vec![], State::Active)]
        );

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Completed), (vec![222], State::Active)]
        );

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Completed), (vec![222], State::Dropped)]
        );
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::stopping_after(2);

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    // The observer stopped on the second window, which the stopped stream then leaves without a
    // termination, as a disposal would.
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .take(2);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker.windows(),
        [(vec![], State::Completed), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

/// The first window is emitted before the source is subscribed to: an observer that stops on it
/// leaves the stream over, so neither the source nor the timer is started.
#[test]
fn test_no_subscriptions_after_initial_window_stops() {
    let time = VirtualTime::new();
    let (_, source, source_checker) = test_channel::<'_, i32, Infallible>();
    let _subscription = source
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .subscribe_with_callback(
            |_| Flow::Stop,
            |_| panic!("a stopped observer must not be terminated"),
        );

    assert_eq!(source_checker.state(), ChannelState::Initialized);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_multiple_operation() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .concat_map(|window| window.to_vec());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111]]);

    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());
    // Unlike `buffer_with_time_or_count`, the completion ends the open window, empty or not, and each window
    // gathered its values while it was open.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![111], vec![], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = WindowWithTimeOrCount::new(
        observable,
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![222], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_revert_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_revert_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_subscribe_window_after_values() {
    let time = VirtualTime::new();
    let mut subject = PublishSubject::<i32, Infallible, _>::shared();
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_cloned = windows.clone();
    let _subscription = subject
        .clone()
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .subscribe_with_callback(
            move |window| windows_cloned.with_mut(|values| values.push(window)),
            |_| {},
        );

    assert!(subject.on_next(111).is_continue());
    assert!(subject.on_next(222).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(subject.on_next(333).is_continue());

    // The windows were collected without being subscribed to, so their items are buffered instead
    // of being dropped.
    let mut windows = windows.take_value().into_iter();
    let (checker_1, observer_1) = Checker::new();
    let _subscription_1 = windows.next().unwrap().subscribe(observer_1);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);

    let (checker_2, observer_2) = Checker::new();
    let _subscription_2 = windows.next().unwrap().subscribe(observer_2);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(windows.next().is_none());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_complete_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.windows(), [(vec![111], State::Error("error"))]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![111], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_after_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    sender.on_termination(Termination::Completed);
    subscription.dispose();
    assert_eq!(checker.windows(), [(vec![], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsub_after_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    sender.on_termination(Termination::Error("error"));
    subscription.dispose();
    assert_eq!(checker.windows(), [(vec![], State::Error("error"))]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

/// The source goes away without a termination: the windows are rotated by time, not by the source,
/// so the open one is still completed at its tick and the ticks go on, with empty windows, until
/// the subscription is disposed.
/// See decision 0004: a source that drops its observer without a termination only stops sending;
/// what the operator has already accepted runs its course.
#[test]
fn test_abandon_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Active)]
    );
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );

    subscription.dispose();
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Dropped),
        ]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 0);
}

/// Disposing after the source went away cancels the ticks: the open window is never completed.
#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![111], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (checker, observer) = WindowChecker::new();

    let observable = source.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    // The first window is open before the source is subscribed to, so it gets the value the source
    // sends while it is being subscribed.
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Active)]
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = WindowChecker::new();

    let observable = Empty.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    // The source completed as it was subscribed to, so the timer was never started.
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_sync_completion_disposes_chain_while_subscription_is_retained() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let source_disposed = Arc::new(AtomicBool::new(false));
    let source_disposed_from_callback = source_disposed.clone();
    let source = Create::shared_boxed(move |mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            source_disposed_from_callback.store(true, Ordering::SeqCst);
        }))
    });
    let (checker, observer) = WindowChecker::new();

    let retained_subscription = source
        .window_with_time_or_count(
            NonZeroUsize::new(10).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .subscribe(observer);

    assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert!(source_disposed.load(Ordering::SeqCst));
    drop(retained_subscription);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = WindowChecker::new();

    let observable = Throw::new("error").window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Error("error"))]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_next_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = WindowChecker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the window.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = WindowChecker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the window.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_error_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = WindowChecker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the window.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);

    subscription.dispose();
    time.advance_by(DURATION_100_MS + DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![], State::Dropped)]);
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
    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
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
    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
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
    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(10).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    observable.filter(|_| true);
}

/// A window filled during the subscription restarts the timer: the first timed rotation comes
/// `time_span` after it, not `time_span` after the subscription.
#[test]
fn test_full_window_during_subscription_restarts_timer() {
    let time = VirtualTime::new();
    let (_, source, _) = test_channel::<'_, _, Infallible>();
    let source = source.hook_on_subscription(|source, observer| {
        time.advance_by(DURATION_30_MS);
        source.start_with([111, 222, 333]).subscribe(observer)
    });
    let (checker, observer) = WindowChecker::new();
    let _subscription = source
        .window_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .subscribe(observer);

    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );
    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );
    time.advance_by(DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Completed),
            (vec![], State::Active),
        ]
    );
}

/// A full window restarts the timer: the first timed rotation comes `time_span` after it, not
/// `time_span` after the subscription.
#[test]
fn test_full_window_restarts_timer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_100_MS - DURATION_30_MS);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );

    // The deadline the full window made obsolete passes without a rotation.
    time.advance_by(DURATION_30_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );

    time.advance_by(DURATION_100_MS - DURATION_30_MS - DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );

    time.advance_by(DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Completed),
            (vec![], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

/// A full window opens the next one at once, so a completion right after it leaves that one empty,
/// as `window_with_count` does.
#[test]
fn test_completed_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![111, 222], State::Completed), (vec![], State::Active)]
    );

    assert!(sender.on_next(333).is_continue());
    assert!(sender.on_next(444).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333, 444], State::Completed),
            (vec![], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333, 444], State::Completed),
            (vec![], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Error("error")),
        ]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

/// A timed rotation starts the count over: the window it opens takes `count` values of its own.
#[test]
fn test_timed_rotation_resets_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, _) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        DURATION_100_MS,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS);
    assert!(sender.on_next(222).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![222], State::Active)]
    );

    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![222, 333], State::Completed),
            (vec![], State::Active),
        ]
    );
}

/// With a time span too long for an `Instant` to represent, the windows are rotated by count only.
#[test]
fn test_time_span_too_long_rotates_by_count() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, _) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_or_count(
        NonZeroUsize::new(2).unwrap(),
        Duration::MAX,
        scheduler.clone(),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Completed)
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// The value that fills a window opens the next one from the source's own send, so the stop that
/// `take` answers on that window reaches the source through that send.
#[test]
fn test_unsub_on_full_window_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable
        .window_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            scheduler.clone(),
        )
        .take(2);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_stop());
    assert_eq!(
        checker.windows(),
        [(vec![111, 222], State::Completed), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

/// A value the consumer of a window sends to the source as the window fills belongs to the next
/// window: the count was reset before the full window was delivered.
#[test]
fn test_reentrant_value_from_window_consumer() {
    let time = VirtualTime::new();
    let (source, observable) = test_channels::<'_, _, Infallible>();
    let reentrant = source.clone();
    let (checker, observer) = WindowChecker::new();
    let _subscription = observable
        .window_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .map(move |window| {
            let reentrant = reentrant.clone();
            window.hook_on_next(move |observer, value| {
                let flow = observer.on_next(value);
                if value == 222 {
                    assert!(reentrant.on_next(0, 333).is_continue());
                }
                flow
            })
        })
        .subscribe(observer);

    assert!(source.on_next(0, 111).is_continue());
    assert!(source.on_next(0, 222).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333], State::Active)
        ]
    );
    assert!(source.on_next(0, 444).is_continue());
    source.on_termination(0, Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![333, 444], State::Completed),
            (vec![], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// A value sent to the source from the completion of a window, timed or full, belongs to the next
/// window, which is emitted after it.
#[test]
fn test_reentrant_value_from_window_completion() {
    for (timed, reentrant_value, next_value) in [(false, 333, 444), (true, 222, 333)] {
        let time = VirtualTime::new();
        let (source, observable) = test_channels::<'_, _, Infallible>();
        let reentrant = source.clone();
        let mut first = true;
        let (checker, observer) = WindowChecker::new();
        let _subscription = observable
            .window_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                DURATION_100_MS,
                time.scheduler(),
            )
            .map(move |window| {
                let reentrant = reentrant.clone();
                let first = std::mem::replace(&mut first, false);
                window.hook_on_termination(move |observer, termination| {
                    observer.on_termination(termination);
                    if first {
                        assert!(reentrant.on_next(0, reentrant_value).is_continue());
                    }
                })
            })
            .subscribe(observer);

        assert!(source.on_next(0, 111).is_continue());
        // Both kinds of completion re-enter the source before emitting the next window.
        let first_values = if timed {
            time.advance_by(DURATION_100_MS);
            vec![111]
        } else {
            assert!(source.on_next(0, 222).is_continue());
            vec![111, 222]
        };
        assert_eq!(
            checker.windows(),
            [
                (first_values.clone(), State::Completed),
                (vec![reentrant_value], State::Active),
            ]
        );
        assert!(source.on_next(0, next_value).is_continue());
        source.on_termination(0, Termination::Completed);
        assert_eq!(
            checker.windows(),
            [
                (first_values, State::Completed),
                (vec![reentrant_value, next_value], State::Completed),
                (vec![], State::Completed),
            ]
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(time.pending_tasks(), 0);
    }
}

#[test]
fn test_full_window_while_timed_rotation_is_delivering() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    for dispose_while_queued in [false, true] {
        let time = Arc::new(VirtualTime::new());
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = WindowChecker::new();
        let (delivering_sender, delivering) = mpsc::channel();
        let (resume_sender, resume) = mpsc::channel();
        let blocked = Arc::new(Mutex::new(Some((delivering_sender, resume))));
        let mut subscription = Some(
            observable
                .window_with_time_or_count(
                    NonZeroUsize::new(2).unwrap(),
                    DURATION_100_MS,
                    time.scheduler(),
                )
                .map(move |window| {
                    let blocked = blocked.clone();
                    window.hook_on_termination(move |observer, termination| {
                        observer.on_termination(termination);
                        if let Some((delivering, resume)) = blocked.take_value() {
                            delivering.send(()).unwrap();
                            resume.recv_timeout(TIMEOUT).unwrap();
                        }
                    })
                })
                .subscribe(observer),
        );

        assert!(sender.on_next(111).is_continue());
        let ticking = time.clone();
        let timer = thread::spawn(move || ticking.advance_by(DURATION_100_MS));
        delivering.recv_timeout(TIMEOUT).unwrap();
        // The timed rotation has reset the model, but its next window is still queued. These
        // values must fill that next window, with the count rotation following them in order.
        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
        if dispose_while_queued {
            subscription.take().unwrap().dispose();
        }
        resume_sender.send(()).unwrap();
        timer.join().unwrap();

        if dispose_while_queued {
            time.advance_by(DURATION_100_MS);
            assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
            assert_eq!(checker.state(), State::Dropped);
        } else {
            let expected = [
                (vec![111], State::Completed),
                (vec![222, 333], State::Completed),
                (vec![], State::Active),
            ];
            assert_eq!(checker.windows(), expected);
            time.advance_by(DURATION_100_MS - DURATION_1_MS);
            assert_eq!(checker.windows(), expected);
            time.advance_by(DURATION_1_MS);
            assert!(sender.on_next(444).is_continue());
            assert_eq!(
                checker.windows(),
                [
                    (vec![111], State::Completed),
                    (vec![222, 333], State::Completed),
                    (vec![], State::Completed),
                    (vec![444], State::Active),
                ]
            );
            subscription.take().unwrap().dispose();
            assert_eq!(checker.windows().last(), Some(&(vec![444], State::Dropped)));
            assert_eq!(checker.state(), State::Dropped);
        }
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(time.pending_tasks(), 0);
    }
}

#[test]
fn test_timed_rotation_while_full_window_is_delivering() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();
    let (delivering_sender, delivering) = mpsc::channel();
    let (resume_sender, resume) = mpsc::channel();
    let blocked = Arc::new(Mutex::new(Some((delivering_sender, resume))));
    let _subscription = observable
        .window_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            time.scheduler(),
        )
        .map(move |window| {
            let blocked = blocked.clone();
            window.hook_on_next(move |observer, value| {
                let flow = observer.on_next(value);
                if value == 222 {
                    let (delivering, resume) = blocked.take_value().unwrap();
                    delivering.send(()).unwrap();
                    resume.recv_timeout(TIMEOUT).unwrap();
                }
                flow
            })
        })
        .subscribe(observer);

    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_100_MS - DURATION_30_MS);
    let source = thread::spawn(move || {
        assert!(sender.on_next(222).is_continue());
        sender
    });
    delivering.recv_timeout(TIMEOUT).unwrap();
    // The full window moved the deadline from 100ms to 170ms before delivering its last value.
    time.advance_by(DURATION_30_MS);
    assert_eq!(checker.windows(), [(vec![111, 222], State::Active)]);
    time.advance_by(DURATION_100_MS - DURATION_30_MS);
    // The 170ms tick queues its rotation behind the count rotation, without delivering it yet.
    assert_eq!(checker.windows(), [(vec![111, 222], State::Active)]);
    resume_sender.send(()).unwrap();
    let mut sender = source.join().unwrap();
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
        ]
    );

    assert!(sender.on_next(333).is_continue());
    assert!(sender.on_next(444).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![], State::Completed),
            (vec![333, 444], State::Completed),
            (vec![], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}
