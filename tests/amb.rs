mod tests_utils;

use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::{
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        conditional_boolean::amb::Amb,
        creating::{empty::Empty, just::Just, throw::Throw},
    },
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Instant,
};

#[test]
fn test_completed() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_without_next() {
    let (sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_without_next() {
    let (sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    subscription.dispose();
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = "error";
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), vec![&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), vec![&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    sender_1.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), vec![&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let (mut sender_1, observable_1, channel_checker_1) =
        test_channel::<'_, &mut i32, Infallible>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2).map(|e| {
        let result = *e;
        *e *= 2;
        result
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&mut value_1).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(&mut value_2).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    drop(channel_checker_1);
    drop(channel_checker_2);
    drop(subscription);
    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (_, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable_1.amb(observable_2);

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(checker.values(), vec![]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        let mut sender_1 = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(111).is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), vec![111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

        let sender_1 = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(222).is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), vec![111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

        scheduler
            .spawn(async move {
                sender_1.on_termination(Termination::<Infallible>::Completed);
            })
            .await;
        assert_eq!(checker.values(), vec![111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker_1.state(), ChannelState::Completed);
        assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels_1, source_1) = test_channels();
    let (channels_2, source_2) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = source_1.amb(source_2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), vec![]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), vec![]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(0, 111).is_continue());
    assert!(channels_1.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), vec![111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), vec![111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Unsubscribed);

    assert!(channels_1.on_next(0, 222).is_continue());
    assert!(channels_1.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), vec![111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), vec![111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Unsubscribed);

    channels_1.on_termination(0, Termination::<Infallible>::Completed);
    channels_1.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), vec![111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), vec![111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1.amb(observable_2).take(1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_stop());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (_, observable_1, channel_checker_1) = test_channel();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (_, observable_4, channel_checker_4) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable_1
        .amb(observable_2)
        .amb(observable_3.amb(observable_4));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Unsubscribed);

    assert!(sender_3.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Unsubscribed);

    sender_3.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
    assert_eq!(channel_checker_4.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (_, observable, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Amb::new_from_iter([
        observable_1.into_send_boxed(),
        observable_2.into_send_boxed(),
        observable.into_send_boxed(),
    ]);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_empty_sources() {
    let (checker, observer) = Checker::<Infallible, Infallible>::new();

    let observable = Amb::new_from_iter(Vec::<Empty>::new());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_three_sources() {
    let (_, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (_, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Amb::new_from_iter([observable_1, observable_2, observable_3]);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(111).is_continue());
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);

    sender_2.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);
}

/// Disposing while the race is open disposes the racing sources the last subscribed first, as the
/// chained disposals of the operators with two sources do.
#[test]
fn test_three_sources_unsubscribe_last_first() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let log = log.clone();
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            let log = log.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                drop(observer);
                log.with_mut(|log| log.push(index));
            }))
        })
    };
    let (checker, observer) = Checker::new();

    let subscription = Amb::new_from_iter([source(1), source(2), source(3)]).subscribe(observer);
    assert!(log.clone_value().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(log.clone_value(), [3, 2, 1]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_three_sources_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (_, observable_2, channel_checker_2) = test_channel();
    let (_, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Amb::new_from_iter([
        source.into_send_boxed(),
        observable_2.into_send_boxed(),
        observable_3.into_send_boxed(),
    ]);

    // The first source wins while it is being subscribed to, so the others are never subscribed.
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);
}

#[test]
fn test_three_sources_tuple() {
    let (_, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (_, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();

    // Sources of different types, raced without boxing.
    let observable = Amb::new((
        observable_1,
        observable_2.map(|value| value * 10),
        observable_3.filter(|value| *value > 0),
    ));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(1).is_continue());
    assert_eq!(checker.values(), vec![10]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);

    assert!(sender_2.on_next(2).is_continue());
    assert_eq!(checker.values(), vec![10, 20]);
    assert_eq!(checker.state(), State::Active);

    sender_2.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![10, 20]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);
}

/// The tuple form disposes the racing sources the last subscribed first, as the list form does.
#[test]
fn test_three_sources_tuple_unsubscribe_last_first() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let log = log.clone();
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            let log = log.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                drop(observer);
                log.with_mut(|log| log.push(index));
            }))
        })
    };
    let (checker, observer) = Checker::new();

    let subscription =
        Amb::new((source(1), source(2).map(|value| value * 10), source(3))).subscribe(observer);
    assert!(log.clone_value().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(log.clone_value(), [3, 2, 1]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_three_sources_tuple_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (_, observable_2, channel_checker_2) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = Amb::new((source, observable_2.map(|value| value * 10), Just::new(333)));

    // The first source wins while it is being subscribed to, so the others are never subscribed:
    // `Just` would otherwise win as it is subscribed.
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (_, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = source.amb(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_later_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (checker, observer) = Checker::new();

    let observable = observable.amb(source);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_next_on_later_sub_disposes_earlier_first() {
    // A source subscribed before the one that wins as it is subscribed is disposed at the win,
    // before the winner's first value is delivered.
    let log = Arc::new(Mutex::new(Vec::new()));
    let earlier = {
        let log = log.clone();
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            let log = log.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                drop(observer);
                log.with_mut(|log| log.push(String::from("disposed")));
            }))
        })
    };
    let (_sender, source, _) = test_channel();
    let source = source.start_with([111]);

    let _subscription = {
        let log = log.clone();
        earlier.amb(source).subscribe_with_callback(
            move |value| log.with_mut(|log| log.push(value.to_string())),
            |_| {},
        )
    };
    assert_eq!(log.clone_value(), ["disposed", "111"]);
}

#[test]
fn test_complete_on_sub() {
    let (_, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Empty.amb(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_later_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, Infallible, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.amb(Empty);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").amb(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.amb(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.amb(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.amb(observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_later_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, Infallible, &str>();
    let (checker, observer) = Checker::new();

    let observable = observable.amb(Throw::new("error"));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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

        let observable = observable.amb(PublishSubject::shared());

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
        let observable = observable.amb(PublishSubject::shared());

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

        let observable = observable.amb(PublishSubject::shared());

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let closure = |mut observer: SendBoxedObserver<'_, _, _>| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    };
    let observable = Create::shared_boxed(closure);
    let observable = observable.amb(Create::shared_boxed(closure));
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    let observable = observable.amb(test_channel::<'_, i32, Infallible>().1);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, Infallible>();

    observable.amb(test_channel::<'_, i32, Infallible>().1);
}

#[test]
fn test_next_on_sub_from_iter() {
    use rx_rust::operators::creating::just::Just;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let generated = AtomicUsize::new(0);
    let sources = (0..).map(|value| {
        generated.fetch_add(1, Ordering::SeqCst);
        Just::new(value).into_shared()
    });
    let (checker, observer) = Checker::new();
    let _subscription = Amb::new_from_iter(sources).subscribe(observer);
    assert_eq!(generated.load(Ordering::SeqCst), 1);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Completed);
}
