mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{
        conditional_boolean::skip_until::SkipUntil,
        creating::{create::Create, just::Just},
    },
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.skip_until(start_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_next(());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(start_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.skip_until(start_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_next(());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(start_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_start_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.skip_until(start_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_start_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.skip_until(start_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_same_source_start_next() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [()]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [(), ()]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(), ()]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_same_source_start_completed() {
    let subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_same_source_start_error() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();
    let mut start_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(start_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(-1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(-2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    start_subject.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    start_subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let mut subject = PublishSubject::default();
    let mut start_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(start_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    start_subject.on_next(());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [&value_2]);
    assert_eq!(checker.state(), State::Active);

    start_subject.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_2]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value_1);
        observer.on_next(&mut value_2);
        observer.on_next(&mut value_3);
        Subscription::default()
    });

    let start_subject = BehaviorSubject::new(());
    let observable = observable.skip_until(start_subject.clone());

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => unreachable!(),
            Termination::Error(error) => assert_eq!(error, "error"),
        },
    );

    start_subject.on_termination(Termination::Error("error"));

    drop(subscription);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let mut start_subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.skip_until(start_subject.clone());

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(-1);
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async move {
                start_subject.on_next(());
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let mut start_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.skip_until(start_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(-1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    start_subject.on_next(());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    start_subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.skip_until(start_observable).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_next(());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let mut start_subject_1 = PublishSubject::default();
    let mut start_subject_2: PublishSubject<'_, (), Infallible> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .skip_until(start_subject_1.clone())
        .skip_until(start_subject_2.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(-1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    start_subject_1.on_next(());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(-1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    start_subject_2.on_next(());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_multiple_operation_same_start() {
    let mut subject = PublishSubject::default();
    let mut start_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .skip_until(start_subject.clone())
        .skip_until(start_subject.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(-1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    start_subject.on_next(());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut start_sender, start_observable, start_channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = SkipUntil::new(observable, start_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(-2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    start_sender.on_next(());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(start_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(start_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<'_, _, Infallible>::new(111);
    let subject_1 = BehaviorSubject::new(());
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().skip_until(subject_1.clone());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let subject_1 = BehaviorSubject::new(());
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Empty.skip_until(subject_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, (), _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Throw::new("error").skip_until(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let start_subject = Create::new(|mut observer| {
            observer.on_next(());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = observable.skip_until(start_subject);

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
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let start_subject = Create::new(|observer: BoxedObserver<'_, (), _>| {
            life_marker_2 = Some(observer);
            Subscription::default()
        });
        let observable = observable.skip_until(start_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_3]);
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
        let observable = Create::new(|observer: BoxedObserver<'_, &TestStruct, Infallible>| {
            life_marker_or = Some(observer);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_sub.consume_ref();
            }))
        });

        let observable = observable.skip_until(Just::new(()));

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let start_subject = Create::new(|_: BoxedObserver<'_, (), _>| Subscription::default());
    let observable = observable.skip_until(start_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let start_subject: PublishSubject<'_, (), _> = PublishSubject::default();
    let observable = subject.skip_until(start_subject);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let start_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.skip_until(start_subject);

    observable.filter(|_| true);
}
