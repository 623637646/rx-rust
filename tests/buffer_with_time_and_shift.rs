mod tests_utils;

use crate::tests_utils::{
    DURATION_1_MS, DURATION_1_YEAR, DURATION_10_MS, DURATION_30_MS, DURATION_100_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel, test_channels},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, empty::Empty, throw::Throw},
        transforming::buffer_with_time_and_shift::BufferWithTimeAndShift,
    },
    scheduler::virtual_time::VirtualTime,
    thread_mode::mutable::MutableExt,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

const SPAN: Duration = DURATION_100_MS;
const HALF: Duration = Duration::from_millis(50);

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    // The second bundle starts half way through the first.
    time.advance_by(HALF);
    assert!(sender.on_next(222).is_continue());
    time.advance_by(HALF - DURATION_1_MS);
    assert!(checker.values().is_empty());

    // The first ends as the third starts.
    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111, 222]]);
    assert!(sender.on_next(333).is_continue());

    time.advance_by(HALF);
    assert_eq!(checker.values(), [vec![111, 222], vec![222, 333]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // The bundles still open are emitted on completion, oldest first, but not the empty one.
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.values(),
        [vec![111, 222], vec![222, 333], vec![333]]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_completed_with_gaps() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(HALF, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert_eq!(checker.values(), [vec![111]]);

    // Between two bundles: dropped.
    assert!(sender.on_next(222).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(333).is_continue());
    time.advance_by(HALF);
    assert_eq!(checker.values(), [vec![111], vec![333]]);

    // An empty bundle is emitted as it ends.
    time.advance_by(SPAN);
    assert_eq!(checker.values(), [vec![111], vec![333], vec![]]);

    time.advance_by(HALF);
    assert!(sender.on_next(444).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![111], vec![333], vec![], vec![444]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_shift_equals_span() {
    let time = VirtualTime::new();
    let (mut sender, observable, _) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(SPAN, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(SPAN);
    assert!(sender.on_next(222).is_continue());
    time.advance_by(SPAN);
    sender.on_termination(Termination::Completed);
    // As `buffer_with_time`: one bundle at a time, each item in one of them.
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_late_timer_catches_up() {
    let time = VirtualTime::new();
    let (_sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
    let source = source.hook_on_subscription(|source, observer| {
        time.advance_by(SPAN + DURATION_30_MS);
        source.start_with([111]).subscribe(observer)
    });
    let (checker, observer) = Checker::new();
    let _subscription = source
        .buffer_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe(observer);

    // The second bundle started at 50 ms, before the value, and the first ended at 100 ms: the
    // times count from the subscription, not from the late timer.
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [vec![111]]);

    time.advance_by(HALF - DURATION_30_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111]]);
    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111], vec![]]);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(SPAN);
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111]]);

    // The error discards the bundles still open.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_time_span_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(Duration::MAX, SPAN, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(SPAN);
    assert!(sender.on_next(222).is_continue());
    // No bundle ever ends, but they keep starting.
    time.advance_by(SPAN * 2);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(time.pending_tasks(), 1);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![111, 222], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

/// A timer that never comes due finishes at once: nothing more can come, so the source dropping
/// its observer drops downstream, with what it holds (decision 0004).
#[test]
fn test_time_span_too_long_abandon_after_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable =
        observable.buffer_with_time_and_shift(Duration::MAX, Duration::MAX, time.scheduler());

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(DURATION_1_YEAR);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(time.pending_tasks(), 0);

    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    time.advance_by(SPAN);
    assert_eq!(checker_1.values(), [vec![111]]);
    assert_eq!(checker_2.values(), [vec![111]]);

    subscription_1.dispose();
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    time.advance_by(HALF);
    assert_eq!(checker_1.values(), [vec![111]]);
    assert_eq!(checker_2.values(), [vec![111], vec![222]]);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111], vec![222], vec![222]]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        let observable = observable.buffer_with_time_and_shift(SPAN, HALF, scheduler.clone());

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
        assert!(checker.values().is_empty());

        // The value arrived before the second bundle started.
        scheduler.sleep(SPAN + DURATION_10_MS).await;
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .buffer_with_time_and_shift(SPAN, HALF, time.scheduler())
        .take(1);

    let _subscription = observable.subscribe(observer);
    time.advance_by(SPAN);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = observable
        .buffer_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(222).is_continue());
    // The bundles still open are emitted on completion, and downstream stops on the first one:
    // the second is not emitted, nor is the completion.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![111, 222]]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

/// A value that arrives between two bundles is dropped outside the lock: here its drop disposes
/// the subscription, which takes that lock again.
#[test]
fn test_drop_value_between_bundles_outside_lock() {
    use crate::tests_utils::drop_probe::{DropCount, DropProbe};

    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::<Vec<Arc<Mutex<DropProbe>>>, _>::new();

    let subscription = Arc::new(Mutex::new(None));
    subscription.replace_value(Some(
        observable
            .buffer_with_time_and_shift(HALF, SPAN, time.scheduler())
            .subscribe(observer),
    ));
    time.advance_by(HALF);
    assert_eq!(checker.state(), State::Active);

    let drops = DropCount::new();
    let subscription_on_drop = subscription.clone();
    let probe = DropProbe::new()
        .on_drop(drops.callback())
        .on_drop(Box::new(move || drop(subscription_on_drop.take_value())));
    let _ = sender.on_next(Arc::new(Mutex::new(probe)));
    assert_eq!(drops.get(), 1);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_drop_original_outside_lock_when_clone_panics() {
    use crate::tests_utils::drop_probe::DropProbe;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct PanickingClone {
        _drop_probe: DropProbe,
    }

    impl Clone for PanickingClone {
        fn clone(&self) -> Self {
            panic!("item cannot be cloned");
        }
    }

    let time = VirtualTime::new();
    let (mut sender, observable, _) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(Some(
        observable
            .buffer_with_time_and_shift(SPAN, HALF, time.scheduler())
            .subscribe(observer),
    )));
    time.advance_by(HALF);

    let subscription_on_drop = subscription.clone();
    let item = PanickingClone {
        _drop_probe: DropProbe::new()
            .on_drop(Box::new(move || drop(subscription_on_drop.take_value()))),
    };
    assert!(catch_unwind(AssertUnwindSafe(|| sender.on_next(item))).is_err());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_abandon_after_next() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(222).is_continue());
    sender.abandon();
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    // The timer goes on cutting the bundles.
    time.advance_by(HALF);
    assert_eq!(checker.values(), [vec![111, 222]]);
    time.advance_by(HALF);
    assert_eq!(checker.values(), [vec![111, 222], vec![222]]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(SPAN);
    assert_eq!(checker.values(), [vec![111, 222], vec![222]]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_after_abandon() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let subscription = observable
        .buffer_with_time_and_shift(SPAN, HALF, time.scheduler())
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    sender.abandon();

    subscription.dispose();
    time.advance_by(SPAN);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_without_convenient_api() {
    let time = VirtualTime::new();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = BufferWithTimeAndShift::new(observable, SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    time.advance_by(HALF);
    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![111, 222], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let time = VirtualTime::new();
    let (sender, source, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable =
        source
            .start_with([111])
            .buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    time.advance_by(SPAN - DURATION_1_MS);
    assert!(checker.values().is_empty());
    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [vec![111]]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::new();

    let observable =
        Empty
            .with_item_type::<i32>()
            .buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_on_sub() {
    let time = VirtualTime::new();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error")
        .with_item_type::<i32>()
        .buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
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
    let observable = BufferWithTimeAndShift::new(observable, SPAN, HALF, time.scheduler());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.buffer_with_time_and_shift(SPAN, HALF, time.scheduler());

    observable.filter(|_| true);
}
