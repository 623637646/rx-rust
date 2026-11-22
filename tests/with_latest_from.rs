mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::combining::with_latest_from::WithLatestFrom,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.with_latest_from(other);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(other_checker.state(), ChannelState::Subscribed);

    // Nothing to pair with yet: dropped.
    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());

    assert!(other_sender.on_next("a").is_continue());
    assert!(checker.values().is_empty());

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [(222, "a")]);

    assert!(other_sender.on_next("b").is_continue());
    assert!(other_sender.on_next("c").is_continue());
    assert_eq!(checker.values(), [(222, "a")]);

    assert!(sender.on_next(333).is_continue());
    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "a"), (333, "c"), (444, "c")]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(222, "a"), (333, "c"), (444, "c")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

/// The largest shape: eight other sources, and an item of nine elements.
#[test]
fn test_completed_eight_sources() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = WithLatestFrom::new(
        observable,
        (
            Just::new(1),
            Just::new("two"),
            Just::new(3.0),
            Just::new('4'),
            Just::new(5u8),
            Just::new(String::from("six")),
            Just::new(7_i64),
            Just::new(true),
        ),
    )
    .subscribe(observer);
    assert!(sender.on_next(0).is_continue());
    assert_eq!(
        checker.values(),
        [(0, 1, "two", 3.0, '4', 5u8, String::from("six"), 7_i64, true)]
    );
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_from_other() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());

    // The other observable's completion keeps its latest value, and the stream goes on.
    other_sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(other_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, "a"), (222, "a")]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_other_are_same() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let _subscription = subject
        .clone()
        .with_latest_from(subject.clone())
        .subscribe(observer);

    // The other observable is subscribed first, so it hears of each value before the source.
    assert!(subject.on_next(111).is_continue());
    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222)]);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());
    assert!(sender.on_next(111).is_continue());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_other() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());
    assert!(sender.on_next(111).is_continue());

    other_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(other_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.with_latest_from(Just::new("a").with_error_type());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [(111, "a")]);
    assert_eq!(checker_2.values(), [(111, "a")]);

    subscription_1.dispose();
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [(111, "a")]);
    assert_eq!(checker_2.values(), [(111, "a"), (222, "a")]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let latest = 222;
    let error = 333;

    let (checker, observer) = Checker::new();
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut other_sender, other, _other_checker) = test_channel();

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next(&latest).is_continue());
    assert!(sender.on_next(&value).is_continue());
    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [(&value, &latest)]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let subscription = observable
        .with_latest_from(Just::new(10).with_error_type())
        .subscribe_with_callback(
            |(value, latest)| {
                on_next(*value + latest);
                *value *= 2;
            },
            on_termination,
        );
    // The disposal names the item type, which borrows `value`.
    drop(subscription);

    assert_eq!(checker.values(), [121]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (mut other_sender, other, other_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.with_latest_from(other);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _senders = scheduler
            .spawn(async move {
                assert!(other_sender.on_next("a").is_continue());
                assert!(sender.on_next(111).is_continue());
                (sender, other_sender)
            })
            .await;
        assert_eq!(checker.values(), [(111, "a")]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [(111, "a")]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.with_latest_from(Just::new("a").with_error_type());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    for index in 0..2 {
        assert!(channels.on_next(index, 111).is_continue());
        channels.on_termination(index, Termination::Error("error"));
    }
    assert_eq!(checker_1.values(), [(111, "a")]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [(111, "a")]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.with_latest_from(other).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());
    assert!(sender.on_next(111).is_break());
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());
    assert!(sender.on_next(111).is_break());
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .with_latest_from(Just::new(10).with_error_type())
        .map(|(value, latest)| value + latest)
        .with_latest_from(Just::new("a").with_error_type());

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(11, "a"), (12, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = WithLatestFrom::new(observable, (Just::new("a").with_error_type(),));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<_, Infallible, _>::shared("a");
    let (mut sender, observable, _) = test_channel();
    let observable = observable.start_with([111]);
    let (checker, observer) = Checker::new();

    // Both emit as they are subscribed, the other observable first, so the source's first value
    // finds the latest one in place.
    let _subscription = observable.with_latest_from(subject).subscribe(observer);
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(111, "a"), (222, "a")]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::<(Infallible, Infallible), Infallible>::new();

    let _subscription = Empty.with_latest_from(Empty).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (_, source, source_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The other observable fails as it is subscribed: the source is not subscribed at all.
    let _subscription = source
        .with_latest_from(Throw::new("error").with_item_type::<i32>())
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(source_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_break());
        }))
    });

    let subscription = observable
        .with_latest_from(Just::new("a").with_error_type())
        .subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let (_, other, other_checker) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let subscription = observable.with_latest_from(other).subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, other, other_checker) = test_channel::<'_, &str, &str>();
    let (checker, observer) = Checker::new();

    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let subscription = observable.with_latest_from(other).subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(other_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_abandon_after_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut other_sender, other, other_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.with_latest_from(other).subscribe(observer);
    assert!(other_sender.on_next("a").is_continue());

    // The other observable lets go: its latest value stays, as for a completion.
    other_sender.abandon();
    assert_eq!(other_checker.state(), ChannelState::Abandoned);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Active);

    // The source lets go: nothing more can come, and the observer is dropped, not terminated.
    sender.abandon();
    assert_eq!(checker.values(), [(111, "a")]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
}

#[test]
fn test_drop_primary_value_outside_lock() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut other_sender, other, _) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .with_latest_from(other)
        .map(|(_, latest)| latest)
        .subscribe(observer);

    // No auxiliary value is ready yet. Dropping the rejected primary value must release the
    // context lock first, since its destructor can provide the missing auxiliary value.
    let drops = DropCount::new();
    let probe = drops.probe().on_drop(Box::new(move || {
        assert!(other_sender.on_next(7).is_continue());
    }));
    assert!(sender.on_next(probe).is_continue());
    assert_eq!(drops.get(), 1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(DropProbe::new()).is_continue());
    assert_eq!(checker.values(), [7]);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_drop_notifier_value_outside_lock() {
    let (mut sender, observable, _channel_checker) = test_channel::<i32, Infallible>();
    let (mut other_sender, other, _other_checker) =
        test_channel::<Arc<Mutex<DropProbe>>, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .with_latest_from(other)
        .map(|(value, _)| value)
        .subscribe(observer);

    // The value a newer one replaces is dropped outside the lock: a drop that re-enters the
    // operator would deadlock otherwise, or panic on the re-entrancy check of a debug build.
    let drops = DropCount::new();
    let probe = DropProbe::new()
        .on_drop(drops.callback())
        .on_drop(Box::new(move || {
            assert!(sender.on_next(111).is_continue());
        }));
    assert!(
        other_sender
            .on_next(Arc::new(Mutex::new(probe)))
            .is_continue()
    );
    assert_eq!(drops.get(), 0);

    assert!(
        other_sender
            .on_next(Arc::new(Mutex::new(DropProbe::new())))
            .is_continue()
    );
    assert_eq!(drops.get(), 1);
    // The value the drop sent was paired with the newer value.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
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
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let other = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(2).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.with_latest_from(other);

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
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.with_latest_from(Just::new(1).with_error_type());

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next((Some(&life_marker_2), 1)).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;
    let (_, mut observer) = Checker::<_, Infallible>::new();

    // Error
    // let (_, mut observer) = Checker::<_, Infallible>::new();
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, _>| {
            drop(observer);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.with_latest_from(Just::new(1).with_error_type());
        assert!(observer.on_next((&life_marker, 1)).is_continue());
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let other = observable.clone();
    let observable = observable.with_latest_from(other);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).with_latest_from(Just::new(2));

    let observable = observable.with_latest_from(Just::new(3));
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Just::new(1).with_latest_from(Just::new(2));

    observable.with_latest_from(Just::new(3));
}

#[test]
fn test_observable_types_without_clone() {
    use rx_rust::observable::ObservableTypes;

    struct NotClone;

    fn inspect_tuple<OE: ObservableTypes<Item = (u8, NotClone)>>(_: OE) {}
    fn inspect_list<OE: ObservableTypes<Item = (u8, Vec<NotClone>)>>(_: OE) {}

    inspect_tuple(WithLatestFrom::new(Just::new(1), (Just::new(NotClone),)));
    inspect_list(WithLatestFrom::new_from_iter(
        Just::new(1),
        [Just::new(NotClone)],
    ));
}

#[test]
fn test_completed_two_others() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, other_1, other_checker_1) = test_channel::<'_, &str, Infallible>();
    let (mut sender_2, other_2, other_checker_2) = test_channel::<'_, f64, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = WithLatestFrom::new(observable, (other_1, other_2)).subscribe(observer);
    assert!(sender_1.on_next("a").is_continue());
    // The second one has not emitted yet: dropped.
    assert!(sender.on_next(1).is_continue());
    assert!(checker.values().is_empty());

    assert!(sender_2.on_next(0.5).is_continue());
    assert!(sender.on_next(2).is_continue());
    assert!(sender_1.on_next("b").is_continue());
    assert!(sender.on_next(3).is_continue());
    assert_eq!(checker.values(), [(2, "a", 0.5), (3, "b", 0.5)]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(other_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(other_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_second_other() {
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (_sender_1, other_1, other_checker_1) = test_channel::<'_, &str, &str>();
    let (sender_2, other_2, _) = test_channel::<'_, f64, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = WithLatestFrom::new(observable, (other_1, other_2)).subscribe(observer);
    sender_2.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(other_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_from_iter() {
    let (mut sender, source, _) = test_channel::<'_, _, Infallible>();
    let mut senders = Vec::new();
    let others: Vec<_> = (0..12)
        .map(|_| {
            let (sender, source, _) = test_channel();
            senders.push(sender);
            source
        })
        .collect();
    let (checker, observer) = Checker::new();
    let _subscription = WithLatestFrom::new_from_iter(source, others).subscribe(observer);
    assert!(sender.on_next("before").is_continue());
    assert!(checker.values().is_empty());
    for (index, sender) in senders.iter_mut().enumerate() {
        assert!(sender.on_next(index).is_continue());
    }
    assert!(sender.on_next("ready").is_continue());
    senders.remove(0).on_termination(Termination::Completed);
    assert!(sender.on_next("after completion").is_continue());
    assert_eq!(
        checker.values(),
        [
            ("ready", (0..12).collect::<Vec<_>>()),
            ("after completion", (0..12).collect()),
        ]
    );
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_empty_from_iter() {
    let (mut sender, source, _) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let _subscription =
        WithLatestFrom::new_from_iter(source, std::iter::empty::<Just<i32>>()).subscribe(observer);
    assert!(sender.on_next("primary").is_continue());
    assert_eq!(checker.values(), [("primary", Vec::new())]);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_order_on_primary_sub_panic() {
    use crate::tests_utils::panic::expect_panic_on_drop;
    use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};

    let order = Arc::new(Mutex::new(Vec::new()));
    let other = |index| {
        let order = order.clone();
        Create::shared_boxed(move |observer| {
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                drop(observer);
                order.with_mut(|order| order.push(index));
            }))
        })
        .with_item_type::<i32>()
        .with_error_type::<Infallible>()
    };
    expect_panic_on_drop(|panic_on_drop| {
        let primary = Create::shared_boxed(move |_| {
            drop(panic_on_drop);
            DisposeOnDrop::<()>::default()
        })
        .with_item_type::<i32>();
        let _subscription = WithLatestFrom::new(primary, (other(0), other(1), other(2)))
            .subscribe_with_callback(|_| {}, |_| {});
    });
    assert_eq!(order.clone_value(), [2, 1, 0]);
}
