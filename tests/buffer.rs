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
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::transforming::buffer::Buffer,
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
    let mut subject = PublishSubject::shared();
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
    let (channels, observable) = test_channels();
    let (boundary_channels, boundary_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(boundary_channels.on_next(0, ()).is_continue());
    assert!(boundary_channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(boundary_channels.on_next(0, ()).is_continue());
    assert!(boundary_channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(boundary_channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(boundary_channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), [Vec::<&_>::new()]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [vec![], vec![&value_1]]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, Infallible>| {
        assert!(observer.on_next(&mut value_1).is_continue());
        assert!(observer.on_next(&mut value_2).is_continue());
        assert!(observer.on_next(&mut value_3).is_continue());
        Subscription::default()
    });

    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let observable = observable.buffer(boundary_observable);

    let subscription = observable.subscribe_with_callback(
        |value| {
            for i in value {
                *i *= 2;
            }
        },
        |_| unreachable!(),
    );

    assert!(boundary_sender.on_next(()).is_continue());
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    drop(subscription);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
    drop(boundary_sender);
    drop(boundary_channel_checker);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer(boundary_observable);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut boundary_sender = scheduler
            .spawn(async move {
                assert!(boundary_sender.on_next(()).is_continue());
                boundary_sender
            })
            .await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let _boundary_sender = scheduler
            .spawn(async move {
                assert!(boundary_sender.on_next(()).is_continue());
                boundary_sender
            })
            .await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (boundary_channels, boundary_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.buffer(boundary_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(boundary_channels.on_next(0, ()).is_continue());
    assert!(boundary_channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [vec![]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(boundary_channels.on_next(0, ()).is_continue());
    assert!(boundary_channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 222).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 333).is_continue());
    assert!(channels.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [vec![], vec![111]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111]]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(boundary_channels.state(1), ChannelState::Subscribed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(boundary_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(boundary_channels.state(1), ChannelState::Unsubscribed);
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
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender_1, boundary_observable_1, boundary_channel_checker_1) = test_channel();
    let (mut boundary_sender_2, boundary_observable_2, boundary_channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .buffer(boundary_observable_1)
        .buffer(boundary_observable_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(boundary_sender_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(boundary_sender_1.on_next(()).is_continue());
    assert_eq!(checker.values(), [Vec::<Vec<_>>::new()]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(boundary_sender_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(boundary_sender_2.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(boundary_sender_1.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![vec![]], vec![]]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(boundary_sender_2.on_next(()).is_continue());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.values(),
        [vec![], vec![vec![]], vec![], vec![vec![111]]]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
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
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(
        boundary_channel_checker_1.state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(
        boundary_channel_checker_2.state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_multiple_operation_same_boundary() {
    let mut subject = PublishSubject::shared();
    let mut boundary_subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .buffer(boundary_subject.clone())
        .buffer(boundary_subject.clone());

    let _subscription = observable.subscribe(observer);
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
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Buffer::new(observable, boundary_observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

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

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let (sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut boundary_subject_sender, boundary_subject_source, _) = test_channel();
    let boundary_subject_source = boundary_subject_source.start_with([()]);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = source.buffer(boundary_subject_source);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [vec![]]);
    assert_eq!(checker.state(), State::Active);

    assert!(boundary_subject_sender.on_next(()).is_continue());
    assert_eq!(checker.values(), [vec![], vec![111]]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
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
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
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
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
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
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let boundary_subject = Create::shared_boxed(|mut observer| {
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
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let boundary_subject = Create::shared_boxed(|observer| {
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
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                life_marker_or_1 = Some(observer);
                Subscription::new(CallbackDisposal::new(|| {
                    life_marker_sub_1.consume_ref();
                }))
            });

        let boundary = Create::shared_boxed(|observer: SendBoxedObserver<'_, (), Infallible>| {
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
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let boundary_subject = Create::shared_boxed(|_| Subscription::default());
    let observable = observable.buffer(boundary_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

/// A buffer is conceptually a window whose items are collected into a `Vec`. This pins down that
/// equivalence on a run that avoids the divergences documented on `Buffer`: the boundary outlives
/// the source, and the pending bundle is not empty when the source completes.
#[test]
fn test_equivalent_to_window_and_collect() {
    let mut subject = PublishSubject::shared();
    let mut boundary_subject = PublishSubject::shared();
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
    let mut subject = PublishSubject::shared();
    let boundary_subject = PublishSubject::shared();
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
    let mut subject = PublishSubject::shared();
    let mut boundary_subject = PublishSubject::shared();
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
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, boundary_observable, _) = test_channel();
    let observable = observable.buffer(boundary_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, boundary_observable, _) = test_channel::<'_, (), String>();
    let observable = observable.buffer(boundary_observable);

    observable.filter(|_| true);
}
