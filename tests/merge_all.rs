mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::TIMEOUT;
use crate::tests_utils::checker::State;
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
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        combining::merge_all::MergeAll,
        creating::{just::Just, throw::Throw},
    },
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Just<i32>, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

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

    let observable = observable.merge_all();

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
    assert_eq!(checker.values(), [111, 222, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Throw<_>, _>();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

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

    let observable = observable.merge_all();

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
    assert_eq!(checker.values(), [111, 222, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    subject_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

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
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe_with_publish_subject() {
    let mut subject = PublishSubject::shared();
    let mut subject_1 = PublishSubject::shared();
    let mut subject_2 = PublishSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();
    let observable = observable.merge_all();
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
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_1.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject_2.on_next(444).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Active);

    subject_2.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let value_4 = 444;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

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

    assert!(sender_2.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(&value_4).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3, &value_4]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3, &value_4]);
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
        DisposeOnDrop::default()
    });
    let observable = observable.merge_all();

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
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.merge_all();

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
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        let _sender_2 = scheduler
            .spawn(async move {
                assert!(sender_2.on_next(222).is_continue());
                sender_2
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (channels_1, source_1) = test_channels();
    let (channels_2, source_2) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.merge_all();
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
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    assert!(channels_2.on_next(0, 222).is_continue());
    assert!(channels_2.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Completed);
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(0, 333).is_continue());
    assert!(channels_1.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    channels_1.on_termination(0, Termination::Completed);
    channels_1.on_termination(1, Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    assert!(channels_2.on_next(0, 444).is_continue());
    assert!(channels_2.on_next(1, 444).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Subscribed);

    channels_2.on_termination(0, Termination::Error("error"));
    channels_2.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Completed);
    assert_eq!(channels_2.state(0), ChannelState::Error("error"));
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
    assert_eq!(channels_2.state(1), ChannelState::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.merge_all().take(1);

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

    assert!(sender_1.on_next(111).is_break());
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

    let observable = observable.merge_all().merge_all();

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

    assert!(sender_2.on_next(observable_4).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_4.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender_3.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
    assert_eq!(channel_checker_4.state(), ChannelState::Subscribed);

    sender_4.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
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

    let observable = MergeAll::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(observable_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

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

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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
    let source_2 = source_2.start_with([333]);
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([source_1]);
    let (checker, observer) = Checker::new();

    let observable = source.merge_all();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(source_2).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(444).is_continue());
    assert_eq!(checker.values(), [111, 222, 333, 444]);
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

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Empty.with_item_type::<Empty>().merge_all();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").with_item_type::<Throw<_>>().merge_all();

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
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                let mut observer = observer;
                assert!(observer.on_next(observable_1).is_break());
            }))
        },
    );

    let observable = observable.merge_all();

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
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        },
    );

    let observable = observable.merge_all();

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
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        },
    );

    let observable = observable.merge_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
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

        let observable = observable.merge_all();

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
                DisposeOnDrop::default()
            },
        );
        let observable = observable.merge_all();

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

        let observable = observable.merge_all();

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
    let observable = observable.merge_all();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, _>();
    let observable = observable.merge_all();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, Infallible>();
    let observable = observable.merge_all();

    observable.filter(|_| true);
}

/// The source drops its observer without a termination (decision 0004): it sends nothing more, but
/// the inner observable it already sent runs its course. The source never completed, so neither
/// does the result: once the inner completes, the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.merge_all().subscribe(observer);
    assert!(sender.on_next(inner).is_continue());
    sender.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(inner_channels.state(0), ChannelState::Subscribed);

    assert!(inner_channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    inner_channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

/// A queued error prevents admission of new inner observables before it reaches downstream.
#[test]
fn test_next_after_error_is_queued() {
    let (mut sender, observable, _) = test_channel();
    let (mut sender_1, observable_1, _) = test_channel();
    let (sender_2, observable_2, channel_checker_2) = test_channel();
    let (_, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();
    let (delivering_sender, delivering) = mpsc::channel();
    let (resume_sender, resume) = mpsc::channel();
    let blocked = Arc::new(Mutex::new(Some((delivering_sender, resume))));

    let observable = observable.merge_all().hook_on_next(move |observer, value| {
        let flow = observer.on_next(value);
        if let Some((delivering, resume)) = blocked.take_value() {
            delivering.send(()).unwrap();
            resume.recv_timeout(TIMEOUT).unwrap();
        }
        flow
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(observable_1).is_continue());
    let deliverer = thread::spawn(move || sender_1.on_next(111).is_break());
    delivering.recv_timeout(TIMEOUT).unwrap();
    assert!(sender.on_next(observable_2).is_continue());
    // Queued behind the delivery of 111, which is still running on the other thread.
    sender_2.on_termination(Termination::Error("error"));
    assert!(sender.on_next(observable_3).is_break());
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    resume_sender.send(()).unwrap();
    assert!(deliverer.join().unwrap());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);
}

/// The same operator with a `max_concurrency`.
mod max_concurrency {
    use crate::tests_utils::DURATION_10_MS;
    use crate::tests_utils::TIMEOUT;
    use crate::tests_utils::checker::State;
    use crate::tests_utils::test_channel::ChannelState;
    use crate::tests_utils::test_channel::ReceiverObservable;
    use crate::tests_utils::test_channel::test_channels;
    use crate::tests_utils::test_scheduler::block_on;
    use crate::tests_utils::{
        checker::Checker, test_channel::test_channel, test_struct::TestStruct,
    };
    use rx_rust::disposable::Disposable;
    use rx_rust::disposable::callback_disposal::CallbackDisposal;
    use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
    use rx_rust::observer::boxed_observer::SendBoxedObserver;
    use rx_rust::operators::creating::create::Create;
    use rx_rust::operators::creating::empty::Empty;
    use rx_rust::subject::publish_subject::PublishSubject;
    use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::{
            combining::merge_all::MergeAll,
            creating::{from_iter::FromIter, just::Just, throw::Throw},
        },
    };
    use std::convert::Infallible;
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::Instant;

    const ONE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    const TWO: NonZeroUsize = NonZeroUsize::new(2).unwrap();
    const THREE: NonZeroUsize = NonZeroUsize::new(3).unwrap();

    #[test]
    fn test_completed_inner_finish() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.merge_all().max_concurrency(TWO);

        let _subscription = observable.subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner.clone()).is_continue());
        // At the limit: the third one waits.
        assert!(sender.on_next(inner.clone()).is_continue());
        assert_eq!(inner_channels.len(), 2);

        assert!(inner_channels.on_next(0, 111).is_continue());
        assert!(inner_channels.on_next(1, 222).is_continue());
        assert_eq!(checker.values(), [111, 222]);

        sender.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);

        // A place is free: the waiting one is subscribed.
        inner_channels.on_termination(0, Termination::Completed);
        assert_eq!(inner_channels.len(), 3);
        assert_eq!(inner_channels.state(2), ChannelState::Subscribed);
        assert!(inner_channels.on_next(2, 333).is_continue());
        assert!(inner_channels.on_next(1, 444).is_continue());
        assert_eq!(checker.values(), [111, 222, 333, 444]);

        inner_channels.on_termination(2, Termination::Completed);
        assert_eq!(checker.state(), State::Active);
        inner_channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_outer_finish() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(TWO)
            .subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner.clone()).is_continue());
        inner_channels.on_termination(0, Termination::Completed);
        inner_channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.state(), State::Active);

        sender.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }

    #[test]
    fn test_completed_queue_order() {
        let (mut sender, observable, _channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        for index in 0..4 {
            assert!(
                sender
                    .on_next(inner.clone().map(move |value| value + index * 100))
                    .is_continue()
            );
        }
        sender.on_termination(Termination::Completed);

        // One at a time, in the order they arrived.
        for index in 0..4 {
            assert_eq!(inner_channels.len(), index + 1);
            assert!(inner_channels.on_next(index, 1).is_continue());
            inner_channels.on_termination(index, Termination::Completed);
        }
        assert_eq!(checker.values(), [1, 101, 201, 301]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_inner_completed_fast() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(sender.on_next(Just::new(111)).is_continue());
        assert!(sender.on_next(Just::new(222)).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }

    #[test]
    fn test_completed_with_synchronous_inner_chain() {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let first = PublishSubject::<i32, Infallible, _>::shared();
        let (checker, observer) = Checker::new();
        let _subscription = subject
            .clone()
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(
            subject
                .on_next(first.clone().into_cloneable_boxed())
                .is_continue()
        );
        for value in 0..10_000 {
            assert!(
                subject
                    .on_next(Just::new(value).into_shared().into_cloneable_boxed())
                    .is_continue()
            );
        }
        subject.on_termination(Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        // Every queued one completes as it is subscribed: the queue is drained in a loop, not by
        // recursion.
        first.on_termination(Termination::Completed);
        assert_eq!(checker.values(), (0..10_000).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_empty() {
        let (sender, observable, channel_checker) =
            test_channel::<'_, ReceiverObservable<'_, i32, Infallible>, _>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(TWO)
            .subscribe(observer);
        sender.on_termination(Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }

    #[test]
    fn test_error_inner_finish() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(inner_channels.on_next(0, 111).is_continue());

        // The error drops the queue: the second one is never subscribed.
        inner_channels.on_termination(0, Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.len(), 1);
    }

    #[test]
    fn test_error_outer_finish() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(inner_channels.on_next(0, 111).is_continue());

        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(inner_channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.len(), 1);
    }

    #[test]
    fn test_error_empty() {
        let (sender, observable, channel_checker) =
            test_channel::<'_, ReceiverObservable<'_, i32, &str>, _>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(TWO)
            .subscribe(observer);
        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    }

    #[test]
    fn test_unsubscribe() {
        let (mut sender, observable, channel_checker) = test_channel();
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let subscription = observable
            .merge_all()
            .max_concurrency(TWO)
            .subscribe(observer);
        for _ in 0..3 {
            assert!(sender.on_next(inner.clone()).is_continue());
        }
        assert!(inner_channels.on_next(0, 111).is_continue());

        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.state(1), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.len(), 2);
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let (mut sender, observable, _channel_checker) = test_channel();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(sender.on_next(observable_1).is_continue());
        assert!(sender_1.on_next(&value).is_continue());
        sender_1.on_termination(Termination::Error(&error));
        assert_eq!(checker.values(), [&value]);
        assert_eq!(checker.state(), State::Error(&error));
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

        let subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe_with_callback(
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
            let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
            let (checker, observer) = Checker::new();

            let observable = observable.merge_all().max_concurrency(ONE);

            let subscription = scheduler
                .spawn(async move { observable.subscribe(observer) })
                .await;

            let inner_channels_sender = inner_channels.clone();
            let _sender = scheduler
                .spawn(async move {
                    assert!(sender.on_next(inner.clone()).is_continue());
                    assert!(sender.on_next(inner).is_continue());
                    assert!(inner_channels_sender.on_next(0, 111).is_continue());
                    inner_channels_sender.on_termination(0, Termination::Completed);
                    sender
                })
                .await;
            assert_eq!(checker.values(), [111]);
            assert_eq!(inner_channels.state(1), ChannelState::Subscribed);

            scheduler.spawn(async { subscription.dispose() }).await;
            scheduler.sleep(DURATION_10_MS).await;
            assert_eq!(checker.values(), [111]);
            assert_eq!(checker.state(), State::Dropped);
            assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
            assert_eq!(inner_channels.state(1), ChannelState::Unsubscribed);
        });
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let (channels, observable) = test_channels::<'_, Just<i32>, Infallible>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let observable = observable.merge_all().max_concurrency(ONE);
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
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.merge_all().max_concurrency(TWO).take(1);

        let _subscription = observable.subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(inner_channels.on_next(1, 111).is_break());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(inner_channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_multiple_operation() {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable
            .map(|value: i32| Just::new(value * 2))
            .merge_all()
            .max_concurrency(ONE)
            .map(|value| Just::new(value + 1))
            .merge_all()
            .max_concurrency(TWO);

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

        let observable = MergeAll::new(observable).max_concurrency(ONE);

        let _subscription = observable.subscribe(observer);
        assert!(sender.on_next(Just::new(111)).is_continue());
        sender.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }

    #[test]
    fn test_next_on_sub() {
        let (mut sender_1, source_1, _) = test_channel();
        let source_1 = source_1.start_with([111]);
        let (mut sender_2, source_2, channel_checker_2) = test_channel();
        let source_2 = source_2.start_with([333]);
        let (mut sender, source, _) = test_channel();
        let source = source.start_with([source_1]);
        let (checker, observer) = Checker::new();

        let _subscription = source.merge_all().max_concurrency(ONE).subscribe(observer);
        assert_eq!(checker.values(), [111]);

        assert!(sender.on_next(source_2).is_continue());
        assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
        assert!(sender_1.on_next(222).is_continue());
        sender_1.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        assert!(sender_2.on_next(444).is_continue());
        sender_2.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_next_on_inner_sub() {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut subject_2 = PublishSubject::<i32, Infallible, _>::shared();
        let (checker, observer) = Checker::new();

        let _subscription = subject
            .clone()
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);

        let mut subject_cloned = subject.clone();
        let subject_2_cloned = subject_2.clone();
        let observable_1 = Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
                // The source emits the next inner while this one is still being subscribed to: it
                // waits for this one, which completes as it is subscribed.
                let _ = observer.on_next(111);
                let _ = subject_cloned.on_next(subject_2_cloned.into_cloneable_boxed());
                observer.on_termination(Termination::Completed);
                DisposeOnDrop::default()
            },
        );

        assert!(
            subject
                .on_next(observable_1.into_cloneable_boxed())
                .is_continue()
        );
        assert_eq!(checker.values(), [111]);

        assert!(subject_2.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
    }

    #[test]
    fn test_next_on_completed_inner_sub_respects_max_concurrency() {
        let (first_sender, first, _) = test_channel::<'_, i32, Infallible>();
        let (mut third_sender, third, third_checker) = test_channel();
        let (fourth_sender, fourth, fourth_checker) = test_channel();
        let (sender, source) = test_channels();
        let (checker, observer) = Checker::new();
        let _subscription = source.merge_all().max_concurrency(ONE).subscribe(observer);

        assert!(sender.on_next(0, first.into_boxed()).is_continue());
        let reentrant_sender = sender.clone();
        let second =
            Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
                observer.on_termination(Termination::Completed);
                // The builder has not returned yet. Another source emission can take the just-freed
                // place, but the builder must not then start the fourth inner as well.
                assert!(
                    reentrant_sender
                        .on_next(0, third.into_boxed())
                        .is_continue()
                );
                assert!(
                    reentrant_sender
                        .on_next(0, fourth.into_boxed())
                        .is_continue()
                );
                DisposeOnDrop::default()
            });
        assert!(sender.on_next(0, second.into_boxed()).is_continue());
        first_sender.on_termination(Termination::Completed);

        assert_eq!(third_checker.state(), ChannelState::Subscribed);
        assert_eq!(fourth_checker.state(), ChannelState::Initialized);
        assert!(third_sender.on_next(3).is_continue());
        third_sender.on_termination(Termination::Completed);
        assert_eq!(fourth_checker.state(), ChannelState::Subscribed);
        fourth_sender.on_termination(Termination::Completed);
        sender.on_termination(0, Termination::Completed);
        assert_eq!(checker.values(), [3]);
        assert_eq!(checker.state(), State::Completed);
    }

    /// A completion after a queued error leaves the stored subscription with the context.
    #[test]
    fn test_complete_after_error_is_queued() {
        let (mut sender, source, _) = test_channel();
        let (mut delivering_sender, delivering_inner, _) = test_channel();
        let (error_sender, error_inner, _) = test_channel();
        let (completing_sender, completing_inner, _) = test_channel();
        let (_, queued_inner, queued_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let (started_sender, started) = mpsc::channel();
        let (resume_sender, resume) = mpsc::channel();
        let (dispose_sender, disposed) = mpsc::channel();
        let completing_inner = completing_inner.do_after_disposal(move || {
            let _ = dispose_sender.send(());
        });

        let _subscription = source
            .merge_all()
            .max_concurrency(THREE)
            .hook_on_next(move |observer, value| {
                let flow = observer.on_next(value);
                started_sender.send(()).unwrap();
                resume.recv_timeout(TIMEOUT).unwrap();
                flow
            })
            .subscribe(observer);
        for inner in [
            delivering_inner.into_boxed(),
            error_inner.into_boxed(),
            completing_inner.into_boxed(),
            queued_inner.into_boxed(),
        ] {
            assert!(sender.on_next(inner).is_continue());
        }
        assert_eq!(queued_checker.state(), ChannelState::Initialized);

        let deliverer = thread::spawn(move || delivering_sender.on_next(111));
        started.recv_timeout(TIMEOUT).unwrap();
        error_sender.on_termination(Termination::Error("error"));
        completing_sender.on_termination(Termination::Completed);
        let queued_state = queued_checker.state();
        let early_disposal = disposed.try_recv();

        resume_sender.send(()).unwrap();
        assert!(deliverer.join().unwrap().is_break());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(queued_state, ChannelState::Initialized);
        assert_eq!(queued_checker.state(), ChannelState::Initialized);
        assert_eq!(early_disposal, Err(mpsc::TryRecvError::Empty));
        assert_eq!(disposed.try_recv(), Ok(()));
    }

    /// A synchronous queue drain also stops if an inner subscription queues an error.
    #[test]
    fn test_error_on_queued_inner_sub() {
        let (sender, source) = test_channels();
        let (mut delivering_sender, delivering_inner, _) = test_channel();
        let (completing_sender, completing_inner, _) = test_channel();
        let (_, queued_inner, queued_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let (started_sender, started) = mpsc::channel();
        let (resume_sender, resume) = mpsc::channel();

        let _subscription = source
            .merge_all()
            .max_concurrency(TWO)
            .hook_on_next(move |observer, value| {
                let flow = observer.on_next(value);
                started_sender.send(()).unwrap();
                resume.recv_timeout(TIMEOUT).unwrap();
                flow
            })
            .subscribe(observer);
        assert!(
            sender
                .on_next(0, delivering_inner.into_boxed())
                .is_continue()
        );
        assert!(
            sender
                .on_next(0, completing_inner.into_boxed())
                .is_continue()
        );
        let error_sender = sender.clone();
        let error_inner =
            Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, &str>| {
                error_sender.on_termination(0, Termination::Error("error"));
                observer.on_termination(Termination::Completed);
                DisposeOnDrop::default()
            });
        assert!(sender.on_next(0, error_inner.into_boxed()).is_continue());
        assert!(sender.on_next(0, queued_inner.into_boxed()).is_continue());

        let deliverer = thread::spawn(move || delivering_sender.on_next(111));
        started.recv_timeout(TIMEOUT).unwrap();
        completing_sender.on_termination(Termination::Completed);
        let queued_state = queued_checker.state();

        resume_sender.send(()).unwrap();
        assert!(deliverer.join().unwrap().is_break());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(queued_state, ChannelState::Initialized);
        assert_eq!(queued_checker.state(), ChannelState::Initialized);
    }

    #[test]
    fn test_complete_on_sub() {
        let (checker, observer) = Checker::<Infallible, Infallible>::new();

        let _subscription = Empty
            .with_item_type::<Empty>()
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error_on_sub() {
        let (checker, observer) = Checker::<Infallible, _>::new();

        let _subscription = Throw::new("error")
            .with_item_type::<Throw<_>>()
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    }

    #[test]
    fn test_next_on_unsub() {
        let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = Create::shared_boxed(
            |observer: SendBoxedObserver<
                '_,
                ReceiverObservable<'_, i32, Infallible>,
                Infallible,
            >| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(observable_1).is_break());
                }))
            },
        );

        let subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
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

        let subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_error_on_unsub() {
        let (checker, observer) = Checker::<i32, _>::new();

        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, Just<i32>, &str>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Error("error"));
                }))
            });

        let subscription = observable
            .map(|inner| inner.with_error_type())
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_abandon_after_next() {
        let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = observable
            .merge_all()
            .max_concurrency(ONE)
            .subscribe(observer);
        assert!(sender.on_next(inner.clone()).is_continue());
        assert!(sender.on_next(inner).is_continue());
        sender.abandon();
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);
        assert_eq!(checker.state(), State::Active);

        // What was accepted runs its course, the queued one included.
        assert!(inner_channels.on_next(0, 111).is_continue());
        inner_channels.on_termination(0, Termination::Completed);
        assert_eq!(inner_channels.state(1), ChannelState::Subscribed);
        assert!(inner_channels.on_next(1, 222).is_continue());
        assert_eq!(checker.state(), State::Active);

        inner_channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
    }

    /// Inner observables completed from several threads at once never have more than the limit
    /// subscribed, and each is subscribed once, in order.
    #[test]
    fn test_race_condition() {
        const ROUNDS: usize = 20;
        const INNERS: usize = 40;
        const THREADS: usize = 4;

        for _ in 0..ROUNDS {
            let (inner_channels, inner) = test_channels::<'_, usize, Infallible>();
            let mut subject = PublishSubject::<_, Infallible, _>::shared();
            let active = Arc::new(AtomicUsize::new(0));
            let max_active = Arc::new(AtomicUsize::new(0));
            let (done_sender, done) = mpsc::channel();
            let (checker, observer) = Checker::new();

            let _subscription = subject
                .clone()
                .merge_all()
                .max_concurrency(THREE)
                .do_after_termination(move |_| done_sender.send(()).unwrap())
                .subscribe(observer);
            for _ in 0..INNERS {
                let (active_up, max_active, active_down) =
                    (active.clone(), max_active.clone(), active.clone());
                let inner = inner
                    .clone()
                    .do_before_subscription(move || {
                        let now = active_up.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(now, Ordering::SeqCst);
                    })
                    .do_before_termination(move |_| {
                        active_down.fetch_sub(1, Ordering::SeqCst);
                    });
                assert!(subject.on_next(inner).is_continue());
            }
            subject.on_termination(Termination::Completed);

            thread::scope(|scope| {
                for thread_index in 0..THREADS {
                    let inner_channels = &inner_channels;
                    scope.spawn(move || {
                        for index in (thread_index..INNERS).step_by(THREADS) {
                            let deadline = Instant::now() + TIMEOUT;
                            while inner_channels.len() <= index
                                || inner_channels.state(index) != ChannelState::Subscribed
                            {
                                assert!(
                                    Instant::now() < deadline,
                                    "inner {index} never subscribed"
                                );
                                thread::yield_now();
                            }
                            assert!(inner_channels.on_next(index, index).is_continue());
                            inner_channels.on_termination(index, Termination::Completed);
                        }
                    });
                }
            });

            done.recv_timeout(TIMEOUT).unwrap();
            assert!(max_active.load(Ordering::SeqCst) <= 3);
            let mut values = checker.values();
            values.sort_unstable();
            assert_eq!(values, (0..INNERS).collect::<Vec<_>>());
            assert_eq!(checker.state(), State::Completed);
        }
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

            let observable = observable.merge_all().max_concurrency(ONE);

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
            let observable = observable.merge_all().max_concurrency(ONE);

            let (_, mut observer) = Checker::new();
            assert!(observer.on_next(&life_marker_2).is_continue());
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
        let observable = observable.merge_all().max_concurrency(ONE);
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        let (_, observable, _) = test_channel::<'_, Just<i32>, _>();
        let observable = observable.merge_all().max_concurrency(ONE);

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::<_, Infallible>::new();
        let _ = observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        let (_, observable, _) = test_channel::<'_, Just<i32>, Infallible>();
        let observable = observable.merge_all().max_concurrency(ONE);

        observable.filter(|_| true);
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_unsubscribe_on_panic_while_subscribing() {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let log = Arc::new(Mutex::new(Vec::new()));
        let observers = Arc::new(Mutex::new(Vec::new()));
        // An inner observable that records its subscription and its disposal in `log`, as `index`
        // and `-index`, and keeps its observer in `observers`; the third one fails to subscribe.
        let inner = {
            let (log, observers) = (log.clone(), observers.clone());
            move |index: i32| {
                let (log, observers) = (log.clone(), observers.clone());
                Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
                    assert_ne!(index, 3, "the subscription fails");
                    log.with_mut(|log| log.push(index));
                    observers.with_mut(|observers| observers.push(Some(observer)));
                    let log = log.clone();
                    DisposeOnDrop::new(CallbackDisposal::new(move || {
                        log.with_mut(|log| log.push(-index));
                    }))
                })
            }
        };
        let (checker, observer) = Checker::new();
        let _subscription = FromIter::new([inner(1), inner(2), inner(3)])
            .merge_all()
            .max_concurrency(TWO)
            .subscribe(observer);
        assert_eq!(log.clone_value(), [1, 2]);

        // The completion subscribes the queued one, which panics: the stream is given up, and the
        // inner observable still running is disposed, rather than left with a place taken for
        // good, so that the stream would never complete.
        let first = observers.with_mut(|observers| observers[0].take().unwrap());
        let result = catch_unwind(AssertUnwindSafe(|| {
            first.on_termination(Termination::Completed);
        }));
        assert!(result.is_err());
        assert_eq!(log.clone_value(), [1, 2, -1, -2]);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_error_on_completed_inner_unsub() {
        let (sender, source) = test_channels();
        let (first_sender, first, _) = test_channel::<'_, i32, &str>();
        let (_, last, last_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let _subscription = source.merge_all().max_concurrency(ONE).subscribe(observer);

        let error_sender = sender.clone();
        let first = first.do_after_disposal(move || {
            error_sender.on_termination(0, Termination::Error("error"));
        });
        assert!(sender.on_next(0, first.into_boxed()).is_continue());
        assert!(sender.on_next(0, last.into_boxed()).is_continue());

        // Disposing the completed inner subscription ends the stream, after the update that
        // reserved the next observable. That subscription starts as decided and is disposed
        // as soon as it returns.
        first_sender.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(last_checker.state(), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_error_on_synchronous_inner_unsub() {
        let (sender, source) = test_channels();
        let (first_sender, first, _) = test_channel::<'_, i32, &str>();
        let (_, last, last_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let _subscription = source.merge_all().max_concurrency(ONE).subscribe(observer);

        let error_sender = sender.clone();
        let completing = Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, &str>| {
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                error_sender.on_termination(0, Termination::Error("error"));
            }))
        });
        assert!(sender.on_next(0, first.into_boxed()).is_continue());
        assert!(sender.on_next(0, completing.into_boxed()).is_continue());
        assert!(sender.on_next(0, last.into_boxed()).is_continue());

        // Disposing the completed inner subscription ends the stream, after the update that
        // reserved the next observable. That subscription starts as decided and is disposed
        // as soon as it returns.
        first_sender.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(last_checker.state(), ChannelState::Unsubscribed);
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_unsubscribe_on_panic_while_releasing() {
        use crate::tests_utils::panic::expect_panic_on_drop;

        let (mut sender, source, source_checker) = test_channel();
        let (first_sender, first, _) = test_channel::<'_, i32, Infallible>();
        let (_, second, second_checker) = test_channel();
        let (_, last, last_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let _subscription = source.merge_all().max_concurrency(TWO).subscribe(observer);

        expect_panic_on_drop(|panic_on_drop| {
            let first = first.do_after_disposal(move || drop(panic_on_drop));
            assert!(sender.on_next(first.into_boxed()).is_continue());
            assert!(sender.on_next(second.into_boxed()).is_continue());
            assert!(sender.on_next(last.into_boxed()).is_continue());
            first_sender.on_termination(Termination::Completed);
        });
        assert_eq!(source_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(second_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(last_checker.state(), ChannelState::Initialized);
        assert_eq!(checker.state(), State::Dropped);
    }
}
