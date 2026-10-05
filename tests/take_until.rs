mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{conditional_boolean::take_until::TakeUntil, creating::just::Just},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (_stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (_stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_stop_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(stop_sender.on_next(()).is_stop());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_stop_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    stop_sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_stop_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    stop_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_same_source_stop_next() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(()).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);

    assert!(subject.on_next(()).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_same_source_stop_completed() {
    let subject: PublishSubject<'_, _, Infallible, _> = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_same_source_stop_error() {
    let subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();
    let observable = observable.take_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels::<'_, _, &str>();
    let (stop_channels, stop_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.take_until(stop_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    assert!(stop_channels.on_next(1, ()).is_stop());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (stop_sender, stop_observable, stop_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    stop_sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value_1).is_continue());
        assert!(observer.on_next(&mut value_2).is_continue());
        assert!(observer.on_next(&mut value_3).is_continue());
        Subscription::default()
    });

    let (stop_sender, stop_observable, stop_channel_checker) = test_channel();
    let observable = observable.take_until(stop_observable);

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => unreachable!(),
            Termination::Error(error) => assert_eq!(error, "error"),
        },
    );

    stop_sender.on_termination(Termination::Error("error"));
    assert_eq!(stop_channel_checker.state(), ChannelState::Error("error"));

    drop(subscription);
    drop(stop_channel_checker);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (_, stop_observable, stop_channel_checker) = test_channel::<'_, (), &str>();
        let (checker, observer) = Checker::new();

        let observable = observable.take_until(stop_observable);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (stop_channels, stop_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.take_until(stop_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Subscribed);

    assert!(stop_channels.on_next(0, ()).is_stop());
    assert!(stop_channels.on_next(1, ()).is_stop());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(stop_channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (_stop_sender, stop_observable, stop_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = observable.take_until(stop_observable).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_stop_1() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut stop_sender_1, stop_observable_1, stop_channel_checker_1) = test_channel();
    let (_, stop_observable_2, _) = test_channel::<'_, (), Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .take_until(stop_observable_1)
        .take_until(stop_observable_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(stop_sender_1.on_next(()).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_stop_2() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (_, stop_observable_1, _) = test_channel::<'_, (), Infallible>();
    let (mut stop_sender_2, stop_observable_2, stop_channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .take_until(stop_observable_1)
        .take_until(stop_observable_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(stop_sender_2.on_next(()).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_same_stop() {
    let mut subject: PublishSubject<'_, _, Infallible, _> = PublishSubject::shared();
    let mut stop_subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();
    let observable = observable
        .take_until(stop_subject.clone())
        .take_until(stop_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(stop_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, &str>();
    let (mut stop_sender, stop_observable, stop_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = TakeUntil::new(observable, stop_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(stop_sender.on_next(()).is_stop());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(stop_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<_, Infallible, _>::shared(111);
    let (_, source_1, _) = test_channel();
    let source_1 = source_1.start_with([()]);
    let (checker, observer) = Checker::new();

    let observable = subject.clone().take_until(source_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = Empty.take_until(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").take_until(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, (), Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.take_until(observable_1);

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
    let (_, observable_1, channel_checker_1) = test_channel::<'_, (), Infallible>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.take_until(observable_1);

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
    let (_, observable_1, channel_checker_1) = test_channel::<'_, (), &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.take_until(observable_1);

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
fn test_lifetime_sub() {
    // OK
    let life_marker_1 = TestStruct;
    let life_marker_2 = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_stop());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let stop_subject = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(()).is_stop());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = observable.take_until(stop_subject);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_3 = TestStruct;
    let mut life_marker_1 = None;
    let mut life_marker_2 = None;

    // Error
    // let mut life_marker_1 = None;
    // let mut life_marker_2 = None;
    // let life_marker_3 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let stop_subject = Create::shared_boxed(|observer: SendBoxedObserver<'_, (), _>| {
            life_marker_2 = Some(observer);
            Subscription::default()
        });
        let observable = observable.take_until(stop_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(vec![&life_marker_3]).is_continue());
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
                Subscription::new(CallbackDisposal::new(|| {
                    life_marker_sub.consume_ref();
                }))
            });

        let observable = observable.take_until(Just::new(()));

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let stop_subject =
        Create::shared_boxed(|_: SendBoxedObserver<'_, (), _>| Subscription::default());
    let observable = observable.take_until(stop_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, stop_observable, _) = test_channel::<'_, (), _>();
    let observable = observable.take_until(stop_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, stop_observable, _) = test_channel::<'_, (), String>();
    let observable = observable.take_until(stop_observable);

    observable.filter(|_| true);
}
