mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::ReceiverObservable;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        combining::exhaust_all::ExhaustAll,
        creating::{just::Just, throw::Throw},
    },
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (_sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.exhaust_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    // The first inner observable is still running: the second one is dropped unsubscribed.
    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(sender_1.on_next(333).is_continue());
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
}

#[test]
fn test_completed_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(111).is_continue());
    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    // Nothing is running any more: the next inner observable is subscribed.
    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert!(sender_2.on_next(222).is_continue());
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_inner_completed_fast() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);

    // Each inner observable completes as it is subscribed, so none is running when the next comes.
    assert!(sender.on_next(Just::new(111)).is_continue());
    assert!(sender.on_next(Just::new(222)).is_continue());
    assert!(sender.on_next(Just::new(333)).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) =
        test_channel::<'_, ReceiverObservable<'_, i32, Infallible>, _>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_same_inner() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let (mut sender, observable, _channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(subject.clone()).is_continue());
    assert!(sender.on_next(subject.clone()).is_continue());

    // The second copy was ignored, so each value arrives once.
    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    subject.on_termination(Termination::Completed);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(111).is_continue());

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(111).is_continue());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_empty() {
    let (sender, observable, channel_checker) =
        test_channel::<'_, ReceiverObservable<'_, i32, &str>, _>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(&value).is_continue());
    sender_1.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut error = -1;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(
            observer
                .on_next(Just::new(&mut value_1).with_error_type())
                .is_continue()
        );
        assert!(
            observer
                .on_next(Just::new(&mut value_2).with_error_type())
                .is_continue()
        );
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });

    let subscription = observable.exhaust_all().subscribe_with_callback(
        |value| *value *= 2,
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => *error *= 2,
        },
    );

    subscription.dispose();
    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(error, -2);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.exhaust_all();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _senders = scheduler
            .spawn(async move {
                assert!(sender.on_next(observable_1).is_continue());
                assert!(sender_1.on_next(111).is_continue());
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
    let (channels, observable) = test_channels::<'_, Just<i32>, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.exhaust_all();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    for index in 0..2 {
        assert!(channels.on_next(index, Just::new(111)).is_continue());
        channels.on_termination(index, Termination::Completed);
    }
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.exhaust_all().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    assert!(sender_1.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .map(|value: i32| Just::new(value * 2))
        .exhaust_all()
        .map(|value| Just::new(value + 1))
        .exhaust_all();

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [3, 5]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = ExhaustAll::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(Just::new(111)).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender_1, source_1, channel_checker_1) = test_channel();
    let source_1 = source_1.start_with([111]);
    let (_sender_2, source_2, channel_checker_2) = test_channel();
    let source_2 = source_2.start_with([333]);
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([source_1]);
    let (checker, observer) = Checker::new();

    let _subscription = source.exhaust_all().subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);

    // Still running: the second one is not subscribed, so its first value is never emitted.
    assert!(sender.on_next(source_2).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::<Infallible>::Completed);
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_inner_sub() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let mut subject_2 = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let _subscription = subject.clone().exhaust_all().subscribe(observer);

    let mut subject_cloned = subject.clone();
    let subject_2_cloned = subject_2.clone();
    let observable_1 = Create::shared_boxed(
        move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
            // The source emits the next inner while this one is still being subscribed to: it is
            // ignored, as this one is running.
            let _ = observer.on_next(111);
            let _ = subject_cloned.on_next(subject_2_cloned.into_send_cloneable_boxed());
            DisposeOnDrop::default()
        },
    );

    assert!(
        subject
            .on_next(observable_1.into_send_cloneable_boxed())
            .is_continue()
    );
    assert_eq!(checker.values(), [111]);

    assert!(subject_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::<Infallible, Infallible>::new();

    let _subscription = Empty
        .with_item_type::<Empty>()
        .exhaust_all()
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::<Infallible, _>::new();

    let observable = Throw::new("error")
        .with_item_type::<Throw<_>>()
        .exhaust_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of being subscribed.
    let observable = Create::shared_boxed(
        |observer: SendBoxedObserver<'_, ReceiverObservable<'_, i32, Infallible>, Infallible>| {
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                let mut observer = observer;
                assert!(observer.on_next(observable_1).is_stop());
            }))
        },
    );

    let subscription = observable.exhaust_all().subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_unsub() {
    let (checker, observer) = Checker::<i32, Infallible>::new();

    let observable =
        Create::shared_boxed(|observer: SendBoxedObserver<'_, Just<i32>, Infallible>| {
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        });

    let subscription = observable.exhaust_all().subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_error_on_unsub() {
    let (checker, observer) = Checker::<i32, _>::new();

    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, Just<i32>, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let subscription = observable
        .map(|inner| inner.with_error_type())
        .exhaust_all()
        .subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_drop_ignored_inner_outside_lock() {
    let (mut sender, observable, _channel_checker) = test_channel();
    let (mut sender_1, observable_1, _channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(observable_1.into_send_boxed()).is_continue());

    // The ignored inner observable is dropped outside the lock: a drop that re-enters the operator
    // would deadlock otherwise, or panic on the re-entrancy check of a debug build.
    let drops = DropCount::new();
    let mut probe = Some(
        DropProbe::new()
            .on_drop(drops.callback())
            .on_drop(Box::new(move || {
                assert!(sender_1.on_next(111).is_continue());
            })),
    );
    let ignored = Create::shared_boxed(move |_observer: SendBoxedObserver<'_, i32, Infallible>| {
        drop(probe.take());
        DisposeOnDrop::default()
    });
    assert!(sender.on_next(ignored.into_send_boxed()).is_continue());
    assert_eq!(drops.get(), 1);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_abandon_after_next() {
    let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.exhaust_all().subscribe(observer);
    assert!(sender.on_next(inner).is_continue());
    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(inner_channels.state(0), ChannelState::Subscribed);

    // The running inner observable goes on, and its completion lets go of the observer.
    assert!(inner_channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    inner_channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
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
            assert!(observer.on_next(Just::new(1)).is_continue());
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.exhaust_all();

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
        let observable = Create::shared_boxed(
            |observer: SendBoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            },
        );
        let observable = observable.exhaust_all();

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
        let observable = Create::shared_boxed(
            |observer: SendBoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_or = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub.consume_ref();
                }))
            },
        );

        let observable = observable.exhaust_all();

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(
            observer
                .on_next(Just::new(TestStruct).with_error_type())
                .is_continue()
        );
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.exhaust_all();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, _>();
    let observable = observable.exhaust_all();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::<_, Infallible>::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, Infallible>();
    let observable = observable.exhaust_all();

    observable.filter(|_| true);
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_on_panic_while_subscribing() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let (mut sender, source, source_checker) = test_channel();
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let _subscription = source.exhaust_all().subscribe(observer);

    // The inner observable fails to subscribe: the stream is given up, rather than left with the
    // slot reserved for good, so that every later inner observable would be dropped and the
    // stream would never complete.
    expect_panic_on_drop(|panic_on_drop| {
        let failing = Create::shared_boxed(move |_: SendBoxedObserver<'_, i32, Infallible>| {
            drop(panic_on_drop);
            DisposeOnDrop::default()
        });
        let _ = sender.on_next(failing.into_send_boxed());
    });
    assert_eq!(source_checker.state(), ChannelState::Unsubscribed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}
