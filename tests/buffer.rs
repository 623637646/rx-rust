mod tests_utils;

use crate::tests_utils::DURATION_NEXT_LOOP;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, transforming::buffer::Buffer},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_first_and_last_empty() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_first_and_last_not_empty() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(0);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![0]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![0]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![0], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![0], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![0], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![0], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_from_boundary() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_boundary_are_same() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [vec![], vec![()]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![()], vec![()]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_last_empty() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_last_not_empty() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_boundary() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(
        boundary_channel_checker.state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer(boundary_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer(boundary_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_1);
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_3);
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::new(|mut observer: BoxedObserver<'_, _, Infallible>| {
        observer.on_next(&mut value_1);
        observer.on_next(&mut value_2);
        observer.on_next(&mut value_3);
        Subscription::default()
    });

    let mut boundary_subject = PublishSubject::default();
    let observable = observable.buffer(boundary_subject.clone());

    let subscription = observable.subscribe_with_callback(
        |value| {
            for i in value {
                *i *= 2;
            }
        },
        |_| unreachable!(),
    );

    boundary_subject.on_next(());

    drop(subscription);
    drop(boundary_subject);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let boundary_subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut boundary_subject_cloned = boundary_subject.clone();
        runtime
            .spawn(async move {
                boundary_subject_cloned.on_next(());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        let mut boundary_subject_cloned = boundary_subject.clone();
        runtime
            .spawn(async move {
                boundary_subject_cloned.on_next(());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(333);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_NEXT_LOOP).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer(boundary_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(0);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(checker.values(), [vec![0]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject_1 = PublishSubject::default();
    let mut boundary_subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .buffer(boundary_subject_1.clone())
        .buffer(boundary_subject_2.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    boundary_subject_2.on_next(());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject_1.on_next(());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject_2.on_next(());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject_2.on_next(());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject_1.on_next(());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject_2.on_next(());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            vec![],
            vec![vec![]],
            vec![],
            vec![vec![111]],
            vec![vec![222, 333]]
        ]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_multiple_operation_same_boundary() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .buffer(boundary_subject.clone())
        .buffer(boundary_subject.clone());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![vec![111], vec![222, 333]]]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Buffer::new(observable, boundary_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_immediate_next() {
    let subject = BehaviorSubject::new(111);
    let mut boundary_subject = BehaviorSubject::new(());
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().buffer(boundary_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_immediate_completed() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Empty.buffer(Empty.map_infallible_to_value());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_immediate_error() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Throw::new("error").buffer(Throw::new("error").map_infallible_to_value());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
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
            Subscription::new_with_disposal_callback(|| {
                life_marker_1.consume_ref();
            })
        });
        let boundary_subject = Create::new(|mut observer| {
            observer.on_next(());
            Subscription::new_with_disposal_callback(|| {
                life_marker_2.consume_ref();
            })
        });
        let observable = observable.buffer(boundary_subject);

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
        let boundary_subject = Create::new(|observer| {
            life_marker_2 = Some(observer);
            Subscription::default()
        });
        let observable = observable.buffer(boundary_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_3]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub_1 = TestStruct;
    let life_marker_sub_2 = TestStruct;

    let mut life_marker_or_1 = None;
    let mut life_marker_or_2 = None;

    // Error
    // let mut life_marker_or_1 = None;
    // let mut life_marker_or_2 = None;

    // let life_marker_sub_1 = TestStruct;
    // let life_marker_sub_2 = TestStruct;

    {
        let observable = Create::new(|observer: BoxedObserver<'_, &TestStruct, Infallible>| {
            life_marker_or_1 = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                life_marker_sub_1.consume_ref();
            })
        });

        let boundary = Create::new(|observer: BoxedObserver<'_, (), Infallible>| {
            life_marker_or_2 = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                life_marker_sub_2.consume_ref();
            })
        });

        let observable = observable.buffer(boundary);

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
    let boundary_subject = Create::new(|_| Subscription::default());
    let observable = observable.buffer(boundary_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject = PublishSubject::default();
    let observable = subject.buffer(boundary_subject);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.buffer(boundary_subject);

    observable.filter(|_| true);
}
