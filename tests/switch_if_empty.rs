mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{conditional_boolean::switch_if_empty::SwitchIfEmpty, creating::just::Just},
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_with_values() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (_fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_completed_without_values() {
    let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Subscribed);

    assert!(fallback_sender.on_next(111).is_continue());
    assert!(fallback_sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);

    fallback_sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_with_values() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (_fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_without_values() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (_fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    // An error is not an empty completion: the fallback is not subscribed.
    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_from_fallback() {
    let (sender, observable, _channel_checker) = test_channel::<'_, i32, _>();
    let (mut fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(fallback_sender.on_next(111).is_continue());
    fallback_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(fallback_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe_with_values() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (_fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let subscription = observable.switch_if_empty(fallback).subscribe(observer);
    assert!(channels.on_next(0, 111).is_continue());

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(fallback_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_unsubscribe_without_values() {
    let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let subscription = observable.switch_if_empty(fallback).subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(fallback_sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(fallback_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (checker, observer) = Checker::new();
    let (sender, observable, channel_checker) = test_channel::<'_, &i32, &i32>();
    let (mut fallback_sender, fallback, fallback_checker) = test_channel();

    let _subscription = observable.switch_if_empty(fallback).subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(fallback_sender.on_next(&value).is_continue());
    fallback_sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::shared_boxed(|observer| {
        observer.on_termination(Termination::Completed);
        DisposeOnDrop::default()
    });
    let fallback = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable
        .switch_if_empty(fallback)
        .subscribe_with_callback(
            |value| {
                on_next(*value);
                *value *= 2;
            },
            |termination| match termination {
                Termination::Completed => panic!(),
                Termination::Error(error) => {
                    on_termination(Termination::Error(*error));
                    *error *= 2;
                }
            },
        );

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (mut fallback_sender, fallback, fallback_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.switch_if_empty(fallback);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _fallback_sender = scheduler
            .spawn(async move {
                sender.on_termination(Termination::Completed);
                assert!(fallback_sender.on_next(111).is_continue());
                fallback_sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(fallback_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.switch_if_empty(Just::new(999).with_error_type());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::Completed);
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [999]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take_with_values() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .switch_if_empty(Just::new(999).with_error_type())
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next_by_take_without_values() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut fallback_sender, fallback, fallback_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.switch_if_empty(fallback).take(1);

    let _subscription = observable.subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(fallback_sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(fallback_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .switch_if_empty(Empty.with_item_type().with_error_type())
        .switch_if_empty(Just::new(111).with_error_type())
        .map(|value| value * 2);

    let _subscription = observable.subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = SwitchIfEmpty::new(observable, Just::new(111).with_error_type());

    let _subscription = observable.subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_unsub() {
    let (checker, observer) = Checker::<_, Infallible>::new();
    let observable = Create::shared_boxed(|observer| {
        observer.on_termination(Termination::Completed);
        DisposeOnDrop::default()
    });
    let fallback = Create::shared_boxed(|mut observer| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            assert!(observer.on_next(111).is_continue());
        }))
    });

    let subscription = observable.switch_if_empty(fallback).subscribe(observer);
    // The fallback holds the observer itself: an item sent from its own disposal is delivered.
    subscription.dispose();
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
        let observable = Create::shared_boxed(|observer| {
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let fallback = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.switch_if_empty(fallback);

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
        let observable =
            observable.switch_if_empty(Create::shared_boxed(|_| DisposeOnDrop::default()));

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(Some(&life_marker_2)).is_continue());
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
    let fallback = observable.clone();
    let observable = observable.switch_if_empty(fallback);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).switch_if_empty(Just::new(2));

    let observable = observable.switch_if_empty(Just::new(3));
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Just::new(1).switch_if_empty(Just::new(2));

    observable.switch_if_empty(Just::new(3));
}
