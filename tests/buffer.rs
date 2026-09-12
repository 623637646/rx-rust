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

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
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

    assert!(sender.on_next(0).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![0]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![0]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![0], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![0], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
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

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
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

    assert!(subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(()).is_continue());
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

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
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

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
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

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
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

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
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

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_3).is_continue());
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
        assert!(observer.on_next(&mut value_1).is_continue());
        assert!(observer.on_next(&mut value_2).is_continue());
        assert!(observer.on_next(&mut value_3).is_continue());
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

    assert!(boundary_subject.on_next(()).is_continue());

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
                assert!(boundary_subject_cloned.on_next(()).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(111).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        let mut boundary_subject_cloned = boundary_subject.clone();
        runtime
            .spawn(async move {
                assert!(boundary_subject_cloned.on_next(()).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(222).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(333).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
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

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
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

    assert!(sender.on_next(0).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_stop());
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

    assert!(boundary_subject_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_1.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_1.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_2.on_next(()).is_continue());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
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

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
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

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::new(111);
    let mut boundary_subject = BehaviorSubject::new(());
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().buffer(boundary_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Empty.buffer(Empty.with_item_type());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Throw::new("error").buffer(Throw::new("error").with_item_type());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, (), Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    // Custom operations
    let observable = observable.buffer(observable_1);

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
    let observable = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    // Custom operations
    let observable = observable.buffer(observable_1);

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
    let observable = Create::new(|observer: BoxedObserver<'_, i32, &str>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    // Custom operations
    let observable = observable.buffer(observable_1);

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
        let observable = Create::new(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let boundary_subject = Create::new(|mut observer| {
            assert!(observer.on_next(()).is_continue());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
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
        assert!(observer.on_next(vec![&life_marker_3]).is_continue());
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
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_sub_1.consume_ref();
            }))
        });

        let boundary = Create::new(|observer: BoxedObserver<'_, (), Infallible>| {
            life_marker_or_2 = Some(observer);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_sub_2.consume_ref();
            }))
        });

        let observable = observable.buffer(boundary);

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let boundary_subject = Create::new(|_| Subscription::default());
    let observable = observable.buffer(boundary_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

/// A buffer is conceptually a window whose items are collected into a `Vec`. This pins down that
/// equivalence on a run that avoids the divergences documented on `Buffer`: the boundary outlives
/// the source, and the pending bundle is not empty when the source completes.
#[test]
fn test_equivalent_to_window_and_collect() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (buffer_checker, buffer_observer) = Checker::new();
    let (window_checker, window_observer) = Checker::new();

    // Custom operations
    let _buffer_subscription = subject
        .clone()
        .buffer(boundary_subject.clone())
        .subscribe(buffer_observer);
    let _window_subscription = subject
        .clone()
        .window(boundary_subject.clone())
        .concat_map(|window| window.to_vec())
        .subscribe(window_observer);

    assert!(boundary_subject.on_next(()).is_continue());
    assert!(subject.on_next(111).is_continue());
    assert!(boundary_subject.on_next(()).is_continue());
    assert!(subject.on_next(222).is_continue());
    assert!(subject.on_next(333).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);

    assert_eq!(buffer_checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(window_checker.values(), buffer_checker.values());
    assert_eq!(buffer_checker.state(), State::Completed);
    assert_eq!(window_checker.state(), State::Completed);
}

/// A completed boundary terminates a buffer, but only stops the rotation of a window, which leaves
/// the composition running until the source terminates.
#[test]
fn test_diverges_from_window_and_collect_on_completed_boundary() {
    let mut subject = PublishSubject::default();
    let boundary_subject = PublishSubject::default();
    let (buffer_checker, buffer_observer) = Checker::new();
    let (window_checker, window_observer) = Checker::new();

    // Custom operations
    let _buffer_subscription = subject
        .clone()
        .buffer(boundary_subject.clone())
        .subscribe(buffer_observer);
    let _window_subscription = subject
        .clone()
        .window(boundary_subject.clone())
        .concat_map(|window| window.to_vec())
        .subscribe(window_observer);

    assert!(subject.on_next(111).is_continue());
    boundary_subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(buffer_checker.values(), [vec![111]]);
    assert_eq!(buffer_checker.state(), State::Completed);
    assert!(window_checker.values().is_empty());
    assert_eq!(window_checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(buffer_checker.values(), [vec![111]]);
    assert_eq!(window_checker.values(), [vec![111, 222]]);
    assert_eq!(window_checker.state(), State::Completed);
}

/// Completing the source with an empty pending bundle emits nothing for a buffer, whereas
/// collecting the empty open window yields a trailing empty `Vec`.
#[test]
fn test_diverges_from_window_and_collect_on_empty_pending_bundle() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (buffer_checker, buffer_observer) = Checker::new();
    let (window_checker, window_observer) = Checker::new();

    // Custom operations
    let _buffer_subscription = subject
        .clone()
        .buffer(boundary_subject.clone())
        .subscribe(buffer_observer);
    let _window_subscription = subject
        .clone()
        .window(boundary_subject.clone())
        .concat_map(|window| window.to_vec())
        .subscribe(window_observer);

    assert!(subject.on_next(111).is_continue());
    assert!(boundary_subject.on_next(()).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);

    assert_eq!(buffer_checker.values(), [vec![111]]);
    assert_eq!(window_checker.values(), [vec![111], vec![]]);
    assert_eq!(buffer_checker.state(), State::Completed);
    assert_eq!(window_checker.state(), State::Completed);
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject = PublishSubject::default();
    let observable = subject.buffer(boundary_subject);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.buffer(boundary_subject);

    observable.filter(|_| true);
}
