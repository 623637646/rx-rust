mod tests_utils;

use crate::tests_utils::{
    DURATION_10_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel, test_channels},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::SendBoxedObserver},
    operators::{
        combining::concat_eager::ConcatEager,
        creating::{create::Create, empty::Empty, just::Just, throw::Throw},
    },
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_eager(observable_1);

    // Both are subscribed at once.
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    // The second one's values are kept until the first has completed.
    assert!(sender_1.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    // Then they go straight through.
    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_later_source_first() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager(observable_1).subscribe(observer);

    // The second one completes first: what it sent waits for the first.
    assert!(sender_1.on_next(222).is_continue());
    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().concat_eager(subject.clone());

    // Both subscriptions get every value: the second's are emitted once the first completes.
    let _subscription = observable.subscribe(observer);
    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender_1.on_next(222).is_continue());
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    // The error of the first ends the stream, and what the second sent is dropped.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111]);

    // An error of a source behind the first ends the stream at once, ahead of its turn.
    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111]);

    // Both are unsubscribed from, and what was kept is dropped.
    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender_1.on_next(&value_2).is_continue());
    assert!(sender.on_next(&value_1).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);

    sender_1.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    let (mut sender, observable, _) = test_channel::<'_, &mut i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel();

    let subscription = observable
        .concat_eager(observable_1)
        .subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |_| panic!(),
        );

    assert!(sender_1.on_next(&mut value_3).is_continue());
    assert!(sender.on_next(&mut value_1).is_continue());
    assert!(sender.on_next(&mut value_2).is_continue());
    sender.on_termination(Termination::Completed);

    drop(subscription);
    drop(sender_1);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.concat_eager(observable_1);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _senders = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(222).is_continue());
                assert!(sender.on_next(111).is_continue());
                (sender, sender_1)
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.clone().concat_eager(observable);
    let observable_2 = observable_1.clone();

    // Channels 0 and 1 are the first subscription's, 2 and 3 the second's.
    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(channels.len(), 4);

    for (first, second, value) in [(0, 1, 111), (2, 3, 333)] {
        assert!(channels.on_next(second, value + 1).is_continue());
        assert!(channels.on_next(first, value).is_continue());
        channels.on_termination(first, Termination::Completed);
        channels.on_termination(second, Termination::Completed);
    }
    assert_eq!(checker_1.values(), [111, 112]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [333, 334]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager(observable_1)
        .take(2)
        .subscribe(observer);
    assert!(sender_1.on_next(222).is_continue());
    assert!(sender_1.on_next(333).is_continue());
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    // Downstream stops at the first of the two values that were kept: the second is not emitted.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe_on_kept_next() {
    use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
    use std::sync::{Arc, Mutex};

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();

    let seen = Arc::new(Mutex::new(Vec::new()));
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();
    let seen_observer = seen.clone();
    let made = observable
        .concat_eager(observable_1)
        .subscribe_with_callback(
            move |value: i32| {
                seen_observer.with_mut(|seen| seen.push(value));
                // Disposed from inside the first of the values that were kept, without stopping.
                if value == 222 {
                    drop(subscription_observer.take_value());
                }
            },
            |_| panic!("a disposed stream does not terminate"),
        );
    subscription.replace_value(Some(made));

    assert!(sender_1.on_next(222).is_continue());
    assert!(sender_1.on_next(333).is_continue());
    assert!(sender.on_next(111).is_continue());

    // The values kept are emitted one after the other, and the disposal is seen in between.
    sender.on_termination(Termination::Completed);
    assert_eq!(seen.clone_value(), [111, 222]);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .map(|value: i32| value * 2)
        .concat_eager(observable_1.filter(|value| value % 2 == 1))
        .map(|value| value + 1)
        .concat_eager(Just::new(0));

    let _subscription = observable.subscribe(observer);
    assert!(sender_1.on_next(3).is_continue());
    assert!(sender_1.on_next(4).is_continue());
    assert!(sender.on_next(1).is_continue());
    assert_eq!(checker.values(), [3]);

    sender.on_termination(Termination::Completed);
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [3, 4, 0]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = ConcatEager::new((observable, Just::new(222)));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut sender_1, source_1, _) = test_channel();
    let source_1 = source_1.start_with([333]);
    let (checker, observer) = Checker::new();

    let _subscription = source.concat_eager(source_1).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(444).is_continue());
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    let _subscription = Empty.concat_eager(Empty).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    // The error ends the stream before the second one is subscribed.
    let _subscription = Throw::new("error")
        .with_item_type()
        .concat_eager(observable_1)
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_break());
        }))
    });

    let subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender_1.on_next(222).is_continue());
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The first completes from inside its own disposal: what the second kept must not be emitted.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender_1.on_next(222).is_continue());
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let subscription = observable.concat_eager(observable_1).subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

/// The first source drops its observer without a termination (decision 0004): it has not
/// completed, so what the second sends is kept for good, and once the second lets go of its
/// observer too the downstream observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager(observable_1).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);

    assert!(sender_1.on_next(222).is_continue());
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.concat_eager(Just::new(2));

        let (_, observer) = Checker::<_, Infallible>::new();
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
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            });
        let observable = observable.concat_eager(Empty.with_item_type());

        let (_, mut observer) = Checker::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub = TestStruct;
    let mut life_marker_or = None;

    // Error
    // let mut life_marker_or = None;
    // let life_marker_sub = TestStruct;

    {
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                life_marker_or = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub.consume_ref();
                }))
            });

        let observable = observable.concat_eager(Empty.with_item_type());

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let another_source = Create::shared_boxed(|_| DisposeOnDrop::default());
    let observable = observable.concat_eager(another_source);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel();
    let observable = observable.concat_eager(another_source_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel();
    let observable = observable.concat_eager(another_source_observable);

    observable.filter(|_| true);
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_on_panic_while_subscribing() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The second source fails to subscribe: the stream is given up, and the first is
    // unsubscribed from.
    expect_panic_on_drop(|panic_on_drop| {
        let failing = Create::shared_boxed(move |_: SendBoxedObserver<'_, i32, Infallible>| {
            drop(panic_on_drop);
            DisposeOnDrop::default()
        });
        let _subscription = observable.concat_eager(failing).subscribe(observer);
    });
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

mod tuple {
    use crate::tests_utils::{
        checker::{Checker, State},
        test_channel::{ChannelState, test_channel},
    };
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::{
            combining::concat_eager::ConcatEager,
            creating::{from_iter::FromIter, just::Just},
        },
    };
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (mut sender_3, observable_3, channel_checker_3) = test_channel();
        let (checker, observer) = Checker::new();

        // Three sources of different types.
        let _subscription = ConcatEager::new((
            observable_1,
            observable_2.map(|value: i32| value * 10),
            observable_3.filter(|value| *value > 0),
        ))
        .subscribe(observer);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

        assert!(sender_3.on_next(3).is_continue());
        sender_3.on_termination(Termination::Completed);
        assert!(sender_2.on_next(2).is_continue());
        assert!(sender_1.on_next(1).is_continue());
        assert_eq!(checker.values(), [1]);

        // The first completes: the second's kept value goes, and the second is now the first.
        sender_1.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [1, 20]);
        assert_eq!(checker.state(), State::Active);

        // The second completes: the third, already completed, is emitted and the stream ends.
        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [1, 20, 3]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_synchronous() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let _subscription = ConcatEager::new((
            FromIter::new([1, 2]),
            Just::new(3),
            FromIter::new([4, 5]).map(|value| value * 10),
        ))
        .subscribe(observer);
        assert_eq!(checker.values(), [1, 2, 3, 40, 50]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_single() {
        let (mut sender, observable, _) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription = ConcatEager::new((observable,)).subscribe(observer);
        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    }
}

mod from_iter {
    use crate::tests_utils::{
        checker::{Checker, State},
        test_channel::{ChannelState, test_channel},
    };
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::{combining::concat_eager::ConcatEager, creating::just::Just},
    };
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let (checker, observer) = Checker::new();
        let _subscription =
            ConcatEager::new_from_iter((0..12).map(|value| Just::new(value).into_shared()))
                .subscribe(observer);
        assert_eq!(checker.values(), (0..12).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_next() {
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription =
            ConcatEager::new_from_iter([observable_1, observable_2]).subscribe(observer);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        assert!(sender_2.on_next(222).is_continue());
        assert!(sender_1.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);

        sender_1.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error() {
        let (_sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription =
            ConcatEager::new_from_iter([observable_1, observable_2]).subscribe(observer);
        assert!(sender_2.on_next(222).is_continue());
        sender_2.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
    }

    #[test]
    fn test_stop() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let generated = AtomicUsize::new(0);
        let sources = (0..).map(|value| {
            generated.fetch_add(1, Ordering::SeqCst);
            Just::new(value).into_shared()
        });
        let (checker, observer) = Checker::new();
        let _subscription = ConcatEager::new_from_iter(sources)
            .take(1)
            .subscribe(observer);
        assert_eq!(generated.load(Ordering::SeqCst), 1);
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_empty() {
        let (checker, observer) = Checker::new();
        let _subscription =
            ConcatEager::new_from_iter(std::iter::empty::<Just<i32>>()).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }
}

mod max_concurrency {
    use crate::tests_utils::{
        checker::{Checker, State},
        test_channel::{ChannelState, test_channel},
    };
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::{combining::concat_eager::ConcatEager, creating::just::Just},
    };
    use std::{convert::Infallible, num::NonZeroUsize};

    const ONE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    const TWO: NonZeroUsize = NonZeroUsize::new(2).unwrap();

    #[test]
    fn test_completed_one_at_a_time() {
        let (mut sender_1, observable_1, _channel_checker_1) = test_channel();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription = observable_1
            .concat_eager(observable_2)
            .max_concurrency(ONE)
            .subscribe(observer);

        // One at a time is `concat`: the second one waits unsubscribed.
        assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
        assert!(sender_1.on_next(111).is_continue());
        sender_1.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        assert!(sender_2.on_next(222).is_continue());
        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_later_source_frees_a_place() {
        let (mut sender_1, observable_1, _channel_checker_1) = test_channel();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (mut sender_3, observable_3, channel_checker_3) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription = ConcatEager::new_from_iter([observable_1, observable_2, observable_3])
            .max_concurrency(TWO)
            .subscribe(observer);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_3.state(), ChannelState::Initialized);

        // The second one completes behind the first: its place goes to the third, and its value
        // stays kept.
        assert!(sender_2.on_next(222).is_continue());
        sender_2.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
        assert!(checker.values().is_empty());

        assert!(sender_3.on_next(333).is_continue());
        assert!(sender_1.on_next(111).is_continue());
        sender_1.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        sender_3.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_endless_iter() {
        let (checker, observer) = Checker::new();

        // A limit pulls an iterator that never ends one source at a time.
        let _subscription =
            ConcatEager::new_from_iter((0..).map(|value| Just::new(value).into_shared()))
                .max_concurrency(TWO)
                .take(5)
                .subscribe(observer);
        assert_eq!(checker.values(), [0, 1, 2, 3, 4]);
        assert_eq!(checker.state(), State::Completed);
    }
}
