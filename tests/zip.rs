mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        combining::zip::Zip,
        creating::{from_iter::FromIter, just::Just},
    },
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_both_empty_first_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_both_empty_second_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_first_empty_first_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_second_empty_second_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_first_some_completed_second_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_second_some_completed_first_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_first_some_completed_second_next() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_break());
    assert_eq!(checker.values(), [(111, "111"), (222, "222"), (333, "333")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_second_some_completed_first_next() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    assert!(sender.on_next(333).is_break());
    assert_eq!(checker.values(), [(111, "111"), (222, "222"), (333, "333")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().zip(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222), (333, 333)]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, 111), (222, 222), (333, 333)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_3_sources() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1).zip(observable_2);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.111).is_continue());
    assert_eq!(checker.values(), [((111, "111"), 0.111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [((111, "111"), 0.111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [((111, "111"), 0.111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.222).is_continue());
    assert_eq!(
        checker.values(),
        [((111, "111"), 0.111), ((222, "222"), 0.222)]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [((111, "111"), 0.111), ((222, "222"), 0.222)]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_break());
    assert_eq!(
        checker.values(),
        [((111, "111"), 0.111), ((222, "222"), 0.222)]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.333).is_break());
    assert_eq!(
        checker.values(),
        [
            ((111, "111"), 0.111),
            ((222, "222"), 0.222),
            ((333, "333"), 0.333)
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_switch() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222"), (333, "333")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("444").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222"), (333, "333")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(
        checker.values(),
        [(111, "111"), (222, "222"), (333, "333"), (444, "444")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [(111, "111"), (222, "222"), (333, "333"), (444, "444")]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().zip(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222), (333, 333)]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, 111), (222, 222), (333, 333)]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (channels_1, source_1) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.zip(source_1);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 222).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(0, "111").is_continue());
    assert!(channels_1.on_next(1, "111").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(1, "222").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(1, "333").is_break());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.values(),
        [(111, "111"), (222, "222"), (333, "333")]
    );
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [(&value_1, &value_3)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [(&value_1, &value_3)]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    let (mut sender, observable, _) = test_channel::<'_, &mut i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel::<'_, &mut i32, _>();

    let observable = observable.zip(observable_1);

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value.0 *= 2;
            *value.1 *= 3;
        },
        |_| unreachable!(),
    );

    assert!(sender.on_next(&mut value_1).is_continue());
    assert!(sender.on_next(&mut value_2).is_continue());
    assert!(sender_1.on_next(&mut value_3).is_continue());

    drop(subscription);
    drop(sender);
    drop(sender_1);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 222);
    assert_eq!(value_3, 999);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.zip(observable_1);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _another_source_sender = scheduler
            .spawn(async move {
                assert!(sender_1.on_next("111").is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (another_source_channels, another_source_observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.zip(another_source_observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    assert!(another_source_channels.on_next(0, "111").is_continue());
    assert!(another_source_channels.on_next(1, "111").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(another_source_channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(another_source_channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.zip(observable_1).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_break());
    assert_eq!(checker.values(), [(111, "111")]);
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
    let (checker, observer) = Checker::new();

    let observable = observable
        .zip(observable_1)
        .zip(observable_2)
        .zip(observable_3);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(true).is_continue());
    assert_eq!(checker.values(), [(((111, "111"), 0.111), true)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(((111, "111"), 0.111), true)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.222).is_continue());
    assert_eq!(checker.values(), [(((111, "111"), 0.111), true)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(false).is_continue());
    assert_eq!(
        checker.values(),
        [
            (((111, "111"), 0.111), true),
            (((222, "222"), 0.222), false)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (((111, "111"), 0.111), true),
            (((222, "222"), 0.222), false)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.333).is_continue());
    assert_eq!(
        checker.values(),
        [
            (((111, "111"), 0.111), true),
            (((222, "222"), 0.222), false)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_3.on_next(true).is_continue());
    assert_eq!(
        checker.values(),
        [
            (((111, "111"), 0.111), true),
            (((222, "222"), 0.222), false)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_break());
    assert_eq!(
        checker.values(),
        [
            (((111, "111"), 0.111), true),
            (((222, "222"), 0.222), false),
            (((333, "333"), 0.333), true)
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation_same_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut another_source_subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = observable
        .zip(another_source_subject.clone())
        .zip(another_source_subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(333).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next("111").is_continue());
    assert_eq!(checker.values(), [(("111", 111), 111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [(("111", 111), 111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next("222").is_continue());
    assert_eq!(checker.values(), [(("111", 111), 111), (("222", 222), 222)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(("111", 111), 111), (("222", 222), 222)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Zip::new((observable, observable_1));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut sender_1, source_1, channel_checker_1) = test_channel();
    let source_1 = source_1.start_with([111]);
    let (checker, observer) = Checker::new();

    let observable = source.zip(source_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![(111, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![(111, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Completed);

    // Zip released the other source when it completed.
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = Empty.zip(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").zip(observable);

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
            assert!(observer.on_next(111).is_break());
        }))
    });

    let observable = observable.zip(observable_1);

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

    let observable = observable.zip(observable_1);

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

    let observable = observable.zip(observable_1);

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
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let another_source_subject = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(()).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = observable.zip(another_source_subject);

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
            DisposeOnDrop::default()
        });
        let another_source_subject = Create::shared_boxed(|observer| {
            life_marker_2 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.zip(another_source_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(
            observer
                .on_next((&life_marker_3, &life_marker_3))
                .is_continue()
        );
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
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_1.consume_ref();
                }))
            });

        let boundary = Create::shared_boxed(|observer: SendBoxedObserver<'_, (), Infallible>| {
            life_marker_or_2 = Some(observer);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_sub_2.consume_ref();
            }))
        });

        let observable = observable.zip(boundary);

        let (_, observer) = Checker::new();
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
    let another_source_subject =
        Create::shared_boxed(|_: SendBoxedObserver<'_, (), TestStruct>| DisposeOnDrop::default());
    let observable = observable.zip(another_source_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel::<'_, String, _>();
    let observable = observable.zip(another_source_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel::<'_, (), String>();
    let observable = observable.zip(another_source_observable);

    observable.filter(|_| true);
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// the values it already sent still pair with those of the other one. Once the other lets go too,
/// nothing more can come, and the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.zip(observable_2).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_continue());
    sender_1.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(10).is_continue());
    assert!(sender_2.on_next(20).is_continue());
    assert_eq!(checker.values(), [(1, 10), (2, 20)]);
    assert_eq!(checker.state(), State::Active);

    sender_2.abandon();
    assert_eq!(checker.values(), [(1, 10), (2, 20)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_2.state(), ChannelState::Abandoned);
}

#[test]
fn test_completed_three_sources() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, &str, Infallible>();
    let (mut sender_3, observable_3, _) = test_channel::<'_, f64, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = Zip::new((observable_1, observable_2, observable_3)).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_continue());
    assert!(sender_2.on_next("a").is_continue());
    assert!(checker.values().is_empty());

    assert!(sender_3.on_next(0.5).is_continue());
    assert_eq!(checker.values(), [(1, "a", 0.5)]);

    // The second source completes with nothing waiting: no other combination can be made.
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a", 0.5)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_eight_sources() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = Zip::new((
        FromIter::new([1, 2]),
        Just::new("two"),
        Just::new(3.0),
        Just::new('4'),
        Just::new(5u8),
        Just::new(String::from("six")),
        Just::new(7_i64),
        Just::new(true),
    ))
    .subscribe(observer);
    assert_eq!(
        checker.values(),
        [(1, "two", 3.0, '4', 5u8, String::from("six"), 7_i64, true)]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsubscribe_order_three_sources() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let order = order.clone();
        Create::shared_boxed(move |_observer: SendBoxedObserver<'_, i32, Infallible>| {
            let order = order.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
            }))
        })
    };

    let subscription =
        Zip::new((source(0), source(1), source(2))).subscribe_with_callback(|_| {}, |_| {});
    subscription.dispose();
    // The last subscribed first.
    assert_eq!(order.clone_value(), [2, 1, 0]);
}

#[test]
fn test_error_on_sub_skips_later_sources() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::<_, &str>::new();

    // The first one fails as it is subscribed: the others are not subscribed at all.
    let _subscription = Zip::new((
        Throw::new("error").with_item_type::<i32>().into_shared(),
        observable,
    ))
    .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_completed_from_iter() {
    use rx_rust::operators::{combining::zip::Zip, creating::from_iter::FromIter};
    let (checker, observer) = Checker::new();
    let sources = (0..12).map(|value| FromIter::new([value, value + 20]).into_shared());
    let _subscription = Zip::new_from_iter(sources).subscribe(observer);
    assert_eq!(
        checker.values(),
        [(0..12).collect::<Vec<_>>(), (20..32).collect()]
    );
    assert_eq!(checker.state(), State::Completed);
}

/// The list form, [`Zip::new_from_iter`].
mod from_iter {
    use crate::tests_utils::DURATION_10_MS;
    use crate::tests_utils::checker::State;
    use crate::tests_utils::test_channel::ChannelState;
    use crate::tests_utils::test_channel::test_channel;
    use crate::tests_utils::test_channel::test_channels;
    use crate::tests_utils::test_scheduler::block_on;
    use crate::tests_utils::{checker::Checker, test_struct::TestStruct};
    use rx_rust::disposable::Disposable;
    use rx_rust::disposable::callback_disposal::CallbackDisposal;
    use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
    use rx_rust::observer::boxed_observer::SendBoxedObserver;
    use rx_rust::operators::creating::create::Create;
    use rx_rust::operators::creating::empty::Empty;
    use rx_rust::operators::creating::from_iter::FromIter;
    use rx_rust::operators::creating::just::Just;
    use rx_rust::operators::creating::throw::Throw;
    use rx_rust::subject::publish_subject::PublishSubject;
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::combining::zip::Zip,
    };
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = Zip::new_from_iter(vec![observable; 3]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(channels.len(), 3);

        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(0, 2).is_continue());
        assert!(channels.on_next(1, 10).is_continue());
        assert!(checker.values().is_empty());

        assert!(channels.on_next(2, 100).is_continue());
        assert_eq!(checker.values(), [vec![1, 10, 100]]);

        assert!(channels.on_next(2, 200).is_continue());
        assert!(channels.on_next(1, 20).is_continue());
        assert_eq!(checker.values(), [vec![1, 10, 100], vec![2, 20, 200]]);

        // The first one completes with nothing left waiting: no other combination can be made.
        channels.on_termination(0, Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
        assert_eq!(channels.state(2), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_completed_after_last_pair() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = Zip::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(0, 2).is_continue());
        channels.on_termination(0, Termination::Completed);
        assert_eq!(checker.state(), State::Active);

        assert!(channels.on_next(1, 10).is_continue());
        assert_eq!(checker.state(), State::Active);
        // The last waiting item of the completed one is paired: nothing more can come.
        assert!(channels.on_next(1, 20).is_break());
        assert_eq!(checker.values(), [vec![1, 10], vec![2, 20]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_completed_empty() {
        let (checker, observer) = Checker::<Vec<i32>, Infallible>::new();

        let _subscription = Zip::new_from_iter(Vec::<Just<i32>>::new()).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_sources_are_same() {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let (checker, observer) = Checker::new();

        let _subscription = Zip::new_from_iter(vec![subject.clone(); 3]).subscribe(observer);
        assert!(subject.on_next(1).is_continue());
        assert!(subject.on_next(2).is_continue());
        assert_eq!(checker.values(), [vec![1, 1, 1], vec![2, 2, 2]]);

        subject.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error() {
        let (channels, observable) = test_channels::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let _subscription = Zip::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 10).is_continue());
        assert!(channels.on_next(0, 2).is_continue());
        channels.on_termination(1, Termination::Error("error"));
        assert_eq!(checker.values(), [vec![1, 10]]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_unsubscribe() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let subscription = Zip::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());

        subscription.dispose();
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_ref() {
        let values = [111, 222];
        let error = 333;

        let (channels, observable) = test_channels();
        let (checker, observer) = Checker::new();

        let _subscription = Zip::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, &values[0]).is_continue());
        assert!(channels.on_next(1, &values[1]).is_continue());
        channels.on_termination(0, Termination::Error(&error));
        assert_eq!(checker.values(), [vec![&values[0], &values[1]]]);
        assert_eq!(checker.state(), State::Error(&error));
    }

    #[test]
    fn test_mut_ref() {
        let mut value_1 = 111;
        let mut value_2 = 222;

        let observable_1 = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(&mut value_1).is_continue());
            DisposeOnDrop::default()
        });
        let observable_2 = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(&mut value_2).is_continue());
            observer.on_termination(Termination::<Infallible>::Completed);
            DisposeOnDrop::default()
        });

        // Not cloned: the items are handed over, `&mut` ones included.
        let subscription =
            Zip::new_from_iter([observable_1.into_boxed(), observable_2.into_boxed()])
                .subscribe_with_callback(
                    |values| values.into_iter().for_each(|value| *value *= 2),
                    |_| {},
                );
        subscription.dispose();
        assert_eq!(value_1, 222);
        assert_eq!(value_2, 444);
    }

    #[test]
    fn test_async() {
        block_on(|scheduler| async move {
            let (channels, observable) = test_channels::<'_, i32, Infallible>();
            let (checker, observer) = Checker::new();

            let observable = Zip::new_from_iter(vec![observable; 2]);

            let subscription = scheduler
                .spawn(async move { observable.subscribe(observer) })
                .await;

            let channels_sender = channels.clone();
            scheduler
                .spawn(async move {
                    assert!(channels_sender.on_next(0, 1).is_continue());
                    assert!(channels_sender.on_next(1, 2).is_continue());
                })
                .await;
            assert_eq!(checker.values(), [vec![1, 2]]);
            assert_eq!(checker.state(), State::Active);

            scheduler.spawn(async { subscription.dispose() }).await;
            scheduler.sleep(DURATION_10_MS).await;
            assert_eq!(checker.values(), [vec![1, 2]]);
            assert_eq!(checker.state(), State::Dropped);
            assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        });
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let (channels, observable) = test_channels::<'_, i32, &str>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let observable_1 = Zip::new_from_iter(vec![observable; 2]);
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        for index in 0..4 {
            assert!(channels.on_next(index, 1).is_continue());
        }
        channels.on_termination(0, Termination::Error("error"));
        channels.on_termination(2, Termination::Error("error"));
        assert_eq!(checker_1.values(), [vec![1, 1]]);
        assert_eq!(checker_1.state(), State::Error("error"));
        assert_eq!(checker_2.values(), [vec![1, 1]]);
        assert_eq!(checker_2.state(), State::Error("error"));
    }

    #[test]
    fn test_unsub_on_next_by_take() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = Zip::new_from_iter(vec![observable; 2]).take(1);

        let _subscription = observable.subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_break());
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_multiple_operation() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let zipped = Zip::new_from_iter([FromIter::new(vec![1, 2]), FromIter::new(vec![3, 4])])
            .map(|values| values.iter().sum::<i32>());
        let observable = Zip::new_from_iter([zipped.clone(), zipped]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![4, 4], vec![6, 6]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_next_on_sub() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let observable = Zip::new_from_iter([
            FromIter::new(vec![1, 2, 3]),
            FromIter::new(vec![10, 20]),
            FromIter::new(vec![100, 200, 300]),
        ]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![1, 10, 100], vec![2, 20, 200]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_complete_on_sub() {
        let (checker, observer) = Checker::<Vec<Infallible>, Infallible>::new();

        let _subscription = Zip::new_from_iter([Empty, Empty]).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error_on_sub() {
        let (channels, observable) = test_channels::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        let sources = [
            Throw::new("error")
                .with_item_type()
                .into_shared()
                .into_boxed(),
            observable.into_boxed(),
        ];

        let _subscription = Zip::new_from_iter(sources).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channels.len(), 0);
    }

    #[test]
    fn test_next_on_unsub() {
        let (checker, observer) = Checker::new();

        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_break());
                }))
            });

        let subscription = Zip::new_from_iter([observable]).subscribe(observer);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_abandon_after_next() {
        let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
        let (mut sender_2, observable_2, _) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = Zip::new_from_iter([observable_1, observable_2]).subscribe(observer);
        assert!(sender_1.on_next(1).is_continue());
        assert!(sender_1.on_next(2).is_continue());
        // Letting go is not a completion: the items it sent are still paired.
        sender_1.abandon();
        assert_eq!(checker.state(), State::Active);

        assert!(sender_2.on_next(10).is_continue());
        assert!(sender_2.on_next(20).is_continue());
        assert!(sender_2.on_next(30).is_continue());
        assert_eq!(checker.values(), [vec![1, 10], vec![2, 20]]);
        assert_eq!(checker.state(), State::Active);

        sender_2.abandon();
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
                assert!(observer.on_next(1).is_continue());
                observer.on_termination(Termination::<String>::Completed);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker.consume_ref();
                }))
            });

            let observable = Zip::new_from_iter([observable]);

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
            let observable = Zip::new_from_iter([observable]);

            let (_, mut observer) = Checker::<_, Infallible>::new();
            assert!(observer.on_next(vec![Some(&life_marker_2)]).is_continue());
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
        let observable = Zip::new_from_iter(vec![observable]);
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        let observable = Zip::new_from_iter([Just::new(1)]);

        let observable = Zip::new_from_iter([observable]);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        let observable = Zip::new_from_iter([Just::new(1)]);

        Zip::new_from_iter([observable]);
    }
}
