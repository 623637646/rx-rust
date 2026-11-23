mod tests_utils;

use crate::tests_utils::{
    DURATION_1_MS, DURATION_1_YEAR, DURATION_10_MS, DURATION_30_MS, DURATION_100_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel, test_channels},
    test_scheduler::block_on,
    test_struct::TestStruct,
    window_checker::WindowChecker,
};
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Flow, Observer, Termination},
    operators::{
        creating::{create::Create, empty::Empty, throw::Throw},
        transforming::window_with_time_and_shift::WindowWithTimeAndShift,
    },
    scheduler::virtual_time::VirtualTime,
};
use std::{convert::Infallible, time::Duration};

const SPAN: Duration = DURATION_100_MS;
const HALF: Duration = Duration::from_millis(50);

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);
    assert!(sender.on_next(111).is_continue());

    // The second window opens half way through the first, and the values go to both.
    time.advance_by(HALF - DURATION_1_MS);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    time.advance_by(DURATION_1_MS);
    assert!(sender.on_next(222).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![111, 222], State::Active), (vec![222], State::Active)]
    );

    // The first is completed as the third opens.
    time.advance_by(HALF);
    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![222, 333], State::Active),
            (vec![333], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // The completion completes every window still open.
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Completed),
            (vec![222, 333], State::Completed),
            (vec![333], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_completed_with_gaps() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(HALF, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert_eq!(checker.windows(), [(vec![111], State::Completed)]);

    // Between two windows: dropped.
    assert!(sender.on_next(222).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![333], State::Active)]
    );

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![333], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

/// A termination between two windows has no window to end, only the outer observable.
#[test]
fn test_completed_between_windows() {
    let time = VirtualTime::new();
    let (sender, observable, _) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(HALF, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    time.advance_by(HALF);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.windows(), [(vec![], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_late_timer_catches_up() {
    let time = VirtualTime::new();
    let (_sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
    let source = source.hook_on_subscription(|source, observer| {
        time.advance_by(SPAN + DURATION_30_MS);
        source.start_with([111]).subscribe(observer)
    });
    let (checker, observer) = WindowChecker::new();
    let _subscription = source
        .window_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe(observer);

    // The windows that were due at 50 ms and 100 ms open, and the first completes, back to back.
    time.advance_by(Duration::ZERO);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Active),
            (vec![], State::Active),
        ]
    );

    time.advance_by(HALF - DURATION_30_MS - DURATION_1_MS);
    assert_eq!(checker.windows().len(), 3);
    time.advance_by(DURATION_1_MS);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Completed),
            (vec![], State::Active),
            (vec![], State::Active),
        ]
    );
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(222).is_continue());

    // The error ends every window still open, then the outer observable.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.windows(),
        [
            (vec![111, 222], State::Error("error")),
            (vec![222], State::Error("error")),
        ]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

/// A time span too long for an `Instant` to represent never ends: the windows stay open until the
/// source terminates, while new ones keep opening.
#[test]
fn test_time_span_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(Duration::MAX, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(SPAN);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Active), (vec![], State::Active)]
    );
    assert_eq!(time.pending_tasks(), 1);

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![], State::Completed)]
    );
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// A timer that never comes due finishes at once: nothing more can come, so the source dropping
/// its observer drops downstream, with what it holds (decision 0004).
#[test]
fn test_time_span_too_long_abandon_after_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable =
        observable.window_with_time_and_shift(Duration::MAX, Duration::MAX, time.scheduler());

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);
    assert_eq!(time.pending_tasks(), 0);

    sender.abandon();
    assert_eq!(checker.windows(), [(vec![111], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    subscription.dispose();
    assert_eq!(checker.windows(), [(vec![111], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = WindowChecker::new();
    let (checker_2, observer_2) = WindowChecker::new();

    let observable_1 = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    time.advance_by(HALF);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());

    // Every open window ends without a termination, and the outer observer is released at once.
    subscription_1.dispose();
    assert_eq!(
        checker_1.windows(),
        [(vec![111], State::Dropped), (vec![111], State::Dropped)]
    );
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.windows(),
        [(vec![111], State::Active), (vec![111], State::Active)]
    );
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(HALF);
    assert_eq!(checker_1.windows().len(), 2);
    assert_eq!(checker_2.windows().len(), 3);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = WindowChecker::new();

        let observable = observable.window_with_time_and_shift(SPAN, HALF, scheduler.clone());

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        let _sender = scheduler
            .spawn(async move {
                let mut sender = sender;
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.windows(), [(vec![111], State::Active)]);

        scheduler.sleep(HALF + DURATION_10_MS).await;
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Active), (vec![], State::Active)]
        );

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(
            checker.windows(),
            [(vec![111], State::Dropped), (vec![], State::Dropped)]
        );
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::stopping_after(2);

    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    // The observer stopped on the second window: the stopped stream leaves both open windows
    // without a termination, as a disposal would.
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Dropped), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable
        .window_with_time_and_shift(SPAN, HALF, time.scheduler())
        .take(2);

    let _subscription = observable.subscribe(observer);
    time.advance_by(HALF);
    assert_eq!(
        checker.windows(),
        [(vec![], State::Dropped), (vec![], State::Dropped)]
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
        .window_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe_with_callback(
            |_| Flow::Break(()),
            |_| panic!("a stopped observer must not be terminated"),
        );

    assert_eq!(source_checker.state(), ChannelState::Initialized);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_abandon_after_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    // The timer goes on opening and completing the windows.
    time.advance_by(SPAN);
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Active),
            (vec![], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(
        checker.windows(),
        [
            (vec![111], State::Completed),
            (vec![], State::Dropped),
            (vec![], State::Dropped),
        ]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();
    let subscription = observable
        .window_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    sender.abandon();

    subscription.dispose();
    time.advance_by(SPAN);
    assert_eq!(checker.windows(), [(vec![1], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = WindowWithTimeAndShift::new(observable, SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    time.advance_by(HALF);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![111], State::Completed), (vec![111], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let (sender, source, _) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let observable =
        source
            .start_with([111])
            .window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![111], State::Active)]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.windows(), [(vec![111], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = WindowChecker::new();

    let observable =
        Empty
            .with_item_type::<i32>()
            .window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = WindowChecker::new();

    let observable = Throw::new("error")
        .with_item_type::<i32>()
        .window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Error("error"))]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    // The constructor does not ask the items to be `Clone`, only subscribing does.
    let observable = WindowWithTimeAndShift::new(observable, SPAN, HALF, time.scheduler());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.window_with_time_and_shift(SPAN, HALF, time.scheduler());

    observable.filter(|_| true);
}
