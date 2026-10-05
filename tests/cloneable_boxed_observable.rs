mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observable::boxed_observable::SendCloneableBoxedObservable;
use rx_rust::operators::creating::create::Create;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{
    checker::{Checker, CheckerObserver},
    test_struct::TestStruct,
};

#[test]
fn test_completed() {
    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = observable.into_send_cloneable_boxed();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = observable.into_send_cloneable_boxed();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.into_send_cloneable_boxed();
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = observable.into_send_cloneable_boxed();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, &value).is_continue());
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channels.state(0), ChannelState::Error(&error));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (channels, observable) = test_channels::<'_, &i32, &str>();
        let (checker, observer) = Checker::new();

        let observable = observable.into_send_cloneable_boxed();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let channels = scheduler
            .spawn(async move {
                assert!(channels.on_next(0, &111).is_continue());
                channels
            })
            .await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable.clone().into_send_cloneable_boxed();
    let observable_2 = observable.clone().into_send_cloneable_boxed();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = observable.into_send_cloneable_boxed().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::<Infallible>::Completed);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = observable
        .clone()
        .into_send_cloneable_boxed()
        .into_send_cloneable_boxed();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (channels, observable) = test_channels();
    let (checker, observer) = Checker::new();

    let observable = SendCloneableBoxedObservable::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
}

#[test]
fn test_fixed_observer() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable
        .clone()
        .into_cloneable_boxed_for::<CheckerObserver<i32, &str>>();
    let observable_2 = observable.into_send_cloneable_boxed_for::<CheckerObserver<i32, &str>>();

    let _subscription_1 = observable_1.clone().subscribe(observer_1);
    let _subscription_2 = observable_2.clone().subscribe(observer_2);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);

    channels.on_termination(0, Termination::Completed);
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
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
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.into_send_cloneable_boxed();

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let life_marker_1 = Arc::new(Mutex::new(None));

    // Error
    // let life_marker_1 = Arc::new(Mutex::new(None));
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker_1.replace_value(Some(observer));
            Subscription::default()
        });
        let observable = observable.into_send_cloneable_boxed();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_oe() {
    // OK
    let life_marker = TestStruct;
    let _observable;

    // Error
    // let _observable;
    // let life_marker = TestStruct;

    {
        let create = Create::shared_boxed(|mut observer| {
            life_marker.consume_ref();
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            Subscription::default()
        });

        _observable = create.into_send_cloneable_boxed();
    }
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable) = test_channels::<'_, i32, String>();
    let observable = observable.into_send_cloneable_boxed();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable) = test_channels::<'_, i32, String>();
    let observable = observable.into_send_cloneable_boxed();

    observable.filter(|_| true);
}
