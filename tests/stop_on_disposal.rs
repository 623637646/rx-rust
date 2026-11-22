mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::callback_observer::CallbackObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::just::Just, utility::stop_on_disposal::StopOnDisposal},
};
use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.stop_on_disposal().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.stop_on_disposal().subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels::<'_, i32, _>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.stop_on_disposal();
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [111]);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 333;

    let (checker, observer) = Checker::new();
    let (mut sender, observable, channel_checker) = test_channel();

    let _subscription = observable.stop_on_disposal().subscribe(observer);
    assert!(sender.on_next(&value).is_continue());
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable
        .stop_on_disposal()
        .subscribe(CallbackObserver::new(
            |value: &mut i32| {
                on_next(*value);
                *value *= 2;
            },
            |termination: Termination<&mut i32>| match termination {
                Termination::Completed => panic!(),
                Termination::Error(error) => {
                    on_termination(Termination::Error(*error));
                    *error *= 2;
                }
            },
        ));

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.stop_on_disposal();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(checker.state(), State::Active);
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
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.stop_on_disposal();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    channels.on_termination(0, Termination::Error("error"));
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.stop_on_disposal().take(1).subscribe(observer);
    assert!(sender.on_next(111).is_break());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::stopping_after(2);

    let _subscription = observable.stop_on_disposal().subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_break());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
}

/// The observer disposes its own subscription and still answers `Flow::Continue(())`: the operator
/// answers `Flow::Break(())` in its place.
#[test]
fn test_unsub_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let (mut on_next, on_termination) = observer.into_callbacks();
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();

    let made = observable
        .stop_on_disposal()
        .subscribe(CallbackObserver::new(
            move |value| {
                on_next(value);
                if value == 222 {
                    drop(subscription_observer.take_value());
                }
            },
            on_termination,
        ));
    subscription.replace_value(Some(made));

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_break());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

/// `take_last` replays its values and completes on one event of its source: a disposal on the
/// first value holds back the others and the completion. Without the operator they all arrive.
#[test]
fn test_unsub_on_next_during_completion() {
    macro_rules! case {
        ($operator:expr, $values:expr, $state:expr) => {{
            let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
            let (checker, observer) = Checker::new();
            let (mut on_next, on_termination) = observer.into_callbacks();
            let subscription = Arc::new(Mutex::new(None));
            let subscription_observer = subscription.clone();
            let made = $operator(observable.take_last(2)).subscribe(CallbackObserver::new(
                move |value| {
                    on_next(value);
                    drop(subscription_observer.take_value());
                },
                on_termination,
            ));
            subscription.replace_value(Some(made));

            for value in [111, 222, 333] {
                assert!(sender.on_next(value).is_continue());
            }
            sender.on_termination(Termination::Completed);
            assert_eq!(checker.values(), $values);
            assert_eq!(checker.state(), $state);
            assert_eq!(channel_checker.state(), ChannelState::Completed);
        }};
    }

    case!(ObservableExt::stop_on_disposal, [222], State::Dropped);
    case!(|observable| observable, [222, 333], State::Completed);
}

/// The disposal is made by a closure further up the chain, which answers no `Flow`, and the
/// observer itself knows nothing of it.
#[test]
fn test_unsub_on_next_by_upstream_callback() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();

    let made = observable
        .buffer_with_count(NonZeroUsize::new(2).unwrap())
        .do_before_next(move |_| drop(subscription_observer.take_value()))
        .stop_on_disposal()
        .subscribe(observer);
    subscription.replace_value(Some(made));

    assert!(sender.on_next(111).is_continue());
    // The last bundle is emitted on completion, and the subscription is disposed before it
    // reaches the observer: neither it nor the completion is delivered.
    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

/// A callback that returns `()` and disposes its subscription gets nothing more, whichever
/// operator still had notifications to send.
#[test]
fn test_unsub_on_next_with_callback() {
    macro_rules! case {
        ($operator:expr, $expected:expr) => {{
            let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
            let (checker, observer) = Checker::new();
            let (mut on_next, on_termination) = observer.into_callbacks();
            let subscription = Arc::new(Mutex::new(None));
            let subscription_observer = subscription.clone();
            let made = $operator(observable)
                .stop_on_disposal()
                .subscribe_with_callback(
                    move |value| {
                        on_next(value);
                        drop(subscription_observer.take_value());
                    },
                    on_termination,
                );
            subscription.replace_value(Some(made));

            for value in [111, 222, 333] {
                assert!(sender.on_next(value).is_continue());
            }
            sender.on_termination(Termination::Completed);
            assert_eq!(checker.values(), $expected);
            assert_eq!(checker.state(), State::Dropped);
            assert_eq!(channel_checker.state(), ChannelState::Completed);
        }};
    }

    case!(|observable| ObservableExt::take_last(observable, 2), [222]);
    case!(ObservableExt::last, [333]);
    case!(ObservableExt::count, [3]);
    case!(ObservableExt::to_vec, [vec![111, 222, 333]]);
    case!(
        |observable| ObservableExt::buffer_with_count(observable, NonZeroUsize::new(4).unwrap()),
        [vec![111, 222, 333]]
    );
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.stop_on_disposal().stop_on_disposal();

    let _subscription = observable.subscribe(observer);
    for value in 0..3 {
        assert!(sender.on_next(value).is_continue());
    }
    assert_eq!(checker.values(), [0, 1, 2]);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = StopOnDisposal::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.stop_on_disposal();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).stop_on_disposal();
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    Just::new(1).stop_on_disposal();
}
