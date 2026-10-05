mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::ReceiverObservable;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        combining::concat_all::ConcatAll,
        creating::{just::Just, throw::Throw},
    },
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_completed_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_completed_inner_completed_fast() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_completed_outer_completed_fast() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Just<i32>, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_same_inner() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut subject_1 = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(subject_1.clone()).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(subject_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(subject_1.clone()).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(subject_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_new_from_iter() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = ConcatAll::new_from_iter([observable_1, observable_2]);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::<i32>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

/// A queue of immediately completing inners is drained by a loop, not nested subscriptions.
#[test]
fn test_completed_with_synchronous_inner_chain() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let first = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().concat_all().subscribe(observer);
    assert!(
        subject
            .on_next(first.clone().into_send_cloneable_boxed())
            .is_continue()
    );
    for value in 0..10_000 {
        assert!(
            subject
                .on_next(Just::new(value).into_shared().into_send_cloneable_boxed())
                .is_continue()
        );
    }
    subject.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    first.on_termination(Termination::Completed);
    assert_eq!(checker.values(), (0..10_000).collect::<Vec<_>>());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_only_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_error_only_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Throw<_>, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_same_inner() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut subject_1 = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(subject_1.clone()).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(subject_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(subject_1.clone()).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(subject_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    subject_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_new_from_iter() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = ConcatAll::new_from_iter([observable_1, observable_2]);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (_, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
}

#[test]
fn test_unsubscribe_with_publish_subject() {
    let mut subject = PublishSubject::shared();
    let mut subject_1 = PublishSubject::shared();
    let mut subject_2 = PublishSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();
    let observable = observable.concat_all();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(subject_1.clone()).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_1.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(subject_2.clone()).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_2.on_next(222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_2.on_next(444).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333, 444]);
    assert_eq!(checker_2.state(), State::Active);

    subject_2.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333, 444]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_3 = 333;
    let value_4 = 444;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(&value_4).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_3, &value_4]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_3, &value_4]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;
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
        assert!(
            observer
                .on_next(Just::new(&mut value_3).with_error_type())
                .is_continue()
        );
        observer.on_termination(Termination::Error(&mut error));
        Subscription::default()
    });
    let observable = observable.concat_all();

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => *error *= 2,
        },
    );
    subscription.dispose();

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
    assert_eq!(error, -2);
}

#[test]
fn test_mut_ref_completed() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;
    let mut completed = false;

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
        assert!(
            observer
                .on_next(Just::new(&mut value_3).with_error_type())
                .is_continue()
        );
        observer.on_termination(Termination::<Infallible>::Completed);
        Subscription::default()
    });
    let observable = observable.concat_all();

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => {
                completed = true;
            }
            Termination::Error(_) => panic!(),
        },
    );
    subscription.dispose();

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
    assert!(completed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
        let (_, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.concat_all();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(observable_1).is_continue());
                sender
            })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _sender_1 = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(111).is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(observable_2).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (channels_1, source_1) = test_channels();
    let (channels_2, source_2) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.concat_all();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, source_1.clone()).is_continue());
    assert!(channels.on_next(1, source_1).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(0, 111).is_continue());
    assert!(channels_1.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, source_2.clone()).is_continue());
    assert!(channels.on_next(1, source_2).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Initialized);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Initialized);

    channels.on_termination(0, Termination::Completed);
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Initialized);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Initialized);

    assert!(channels_1.on_next(0, 333).is_continue());
    assert!(channels_1.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [111, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Initialized);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Initialized);

    channels_1.on_termination(0, Termination::Completed);
    channels_1.on_termination(1, Termination::Completed);
    assert_eq!(checker_1.values(), [111, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    assert!(channels_2.on_next(0, 444).is_continue());
    assert!(channels_2.on_next(1, 444).is_continue());
    assert_eq!(checker_1.values(), [111, 333, 444]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 333, 444]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    channels_2.on_termination(0, Termination::Error("error"));
    channels_2.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 333, 444]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Error("error"));
    assert_eq!(checker_2.values(), [111, 333, 444]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (mut sender_4, observable_4, channel_checker_4) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all().concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_2).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(observable_3).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(observable_4).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_3.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);

    assert!(sender_4.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender_4.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
    assert_eq!(channel_checker_4.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = ConcatAll::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_2).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender_1, source_1, _) = test_channel();
    let source_1 = source_1.start_with([111]);
    let (mut sender_2, source_2, _) = test_channel();
    let source_2 = source_2.start_with([444]);
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([source_1]);
    let (checker, observer) = Checker::new();

    let observable = source.concat_all();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(source_2).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_2.on_next(555).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Active);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444, 555]);
    assert_eq!(checker.state(), State::Completed);
}

/// The source emits, from another thread, while an inner observable that terminates synchronously
/// is being subscribed to by the completion of the previous one: the value it emits must be
/// queued behind the subscription in flight, not race it for the slot.
#[test]
fn test_next_on_inner_sub_from_another_thread() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let mut subject_1 = PublishSubject::<i32, Infallible, _>::shared();
    let mut subject_3 = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().concat_all();

    let _subscription = observable.subscribe(observer);

    assert!(
        subject
            .on_next(subject_1.clone().into_send_cloneable_boxed())
            .is_continue()
    );
    let subject_cloned = subject.clone();
    let subject_3_cloned = subject_3.clone();
    let observable_2 =
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            // Terminates synchronously, and only then lets the source emit the next observable,
            // from a thread the source's own delivery does not serialize with this subscription.
            observer.on_termination(Termination::Completed);
            let mut subject_cloned = subject_cloned;
            std::thread::spawn(move || {
                assert!(
                    subject_cloned
                        .on_next(subject_3_cloned.into_send_cloneable_boxed())
                        .is_continue()
                );
            })
            .join()
            .unwrap();
            Subscription::default()
        });
    assert!(
        subject
            .on_next(observable_2.into_send_cloneable_boxed())
            .is_continue()
    );
    // This queued inner must run before the one emitted during observable_2.subscribe().
    assert!(
        subject
            .on_next(Just::new(222).into_shared().into_send_cloneable_boxed())
            .is_continue()
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    // Completing the first inner subscribes to the second, which completes at once while the
    // third arrives: the third is subscribed to next, in order.
    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject_3.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    subject_3.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

/// The source emits, re-entrantly, while an inner observable that terminates synchronously is
/// being subscribed to: that value must be queued behind the subscription in flight, not race it
/// for the slot. This is `test_next_on_sub` for the *inner* subscription.
#[test]
fn test_next_on_inner_sub() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let subject_1 = PublishSubject::<i32, Infallible, _>::shared();
    let mut subject_3 = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().concat_all();
    let _subscription = observable.subscribe(observer);

    assert!(
        subject
            .on_next(subject_1.clone().into_send_cloneable_boxed())
            .is_continue()
    );

    let mut subject_cloned = subject.clone();
    let subject_3_cloned = subject_3.clone();
    let observable_2 =
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            // Terminates synchronously, and only then lets the source emit the next observable — on
            // this very thread, from inside the first inner observable's own termination.
            observer.on_termination(Termination::Completed);
            let _ = subject_cloned.on_next(subject_3_cloned.into_send_cloneable_boxed());
            Subscription::default()
        });
    assert!(
        subject
            .on_next(observable_2.into_send_cloneable_boxed())
            .is_continue()
    );

    assert!(
        subject
            .on_next(Just::new(222).into_shared().into_send_cloneable_boxed())
            .is_continue()
    );

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject_3.on_next(333).is_continue());
    assert_eq!(checker.values(), [222, 333]);
    subject.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    subject_3.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Empty.with_item_type::<Empty>().concat_all();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

/// Source completion during an inner build must wait for its outstanding reservation to finish.
#[test]
fn test_complete_on_inner_sub() {
    let mut subject = PublishSubject::<_, Infallible, _>::shared();
    let first = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().concat_all().subscribe(observer);
    assert!(
        subject
            .on_next(first.clone().into_send_cloneable_boxed())
            .is_continue()
    );

    let source = subject.clone();
    let checker_during_sub = checker.clone();
    let inner = Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
        observer.on_termination(Termination::Completed);
        source.on_termination(Termination::Completed);
        // The builder still owns the continuation until it returns its subscription.
        assert_eq!(checker_during_sub.state(), State::Active);
        Subscription::default()
    });
    assert!(
        subject
            .on_next(inner.into_send_cloneable_boxed())
            .is_continue()
    );

    first.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error")
        .with_item_type::<Throw<_>>()
        .concat_all();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(
        |observer: SendBoxedObserver<'_, ReceiverObservable<'_, i32, Infallible>, Infallible>| {
            Subscription::new(CallbackDisposal::new(move || {
                let mut observer = observer;
                assert!(observer.on_next(observable_1).is_stop());
            }))
        },
    );

    let observable = observable.concat_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_unsub() {
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(
        |observer: SendBoxedObserver<'_, ReceiverObservable<'_, i32, Infallible>, Infallible>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        },
    );

    let observable = observable.concat_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_error_on_unsub() {
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(
        |observer: SendBoxedObserver<'_, ReceiverObservable<'_, i32, &str>, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        },
    );

    let observable = observable.concat_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_inner_observer_pending_subscription_when_terminated() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.concat_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    assert!(sender.on_next(observable_1.into_send_boxed()).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(
        sender
            .on_next(Just::new(-1).into_shared().into_send_boxed())
            .is_continue()
    );
    assert!(
        sender
            .on_next(Just::new(-2).into_shared().into_send_boxed())
            .is_continue()
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, -1, -2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
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
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.concat_all();

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
        let observable = Create::shared_boxed(
            |observer: SendBoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_1 = Some(observer);
                Subscription::default()
            },
        );
        let observable = observable.concat_all();

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
                Subscription::new(CallbackDisposal::new(|| {
                    life_marker_sub.consume_ref();
                }))
            },
        );

        let observable = observable.concat_all();

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
        Subscription::default()
    });
    let observable = observable.concat_all();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, _>();
    let observable = observable.concat_all();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, Infallible>();
    let observable = observable.concat_all();

    observable.filter(|_| true);
}
