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
    operators::{combining::combine_latest::CombineLatest, creating::just::Just},
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

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
    assert_eq!(checker.values(), [(222, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111"), (444, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    assert!(sender.on_next(555).is_continue());
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (555, "222")
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (555, "222")
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

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
    assert_eq!(checker.values(), [(222, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111"), (444, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("333").is_continue());
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (444, "333")
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (444, "333")
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().combine_latest(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(
        checker.values(),
        [(111, 111), (222, 111), (222, 222), (333, 222), (333, 333)]
    );
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [(111, 111), (222, 111), (222, 222), (333, 222), (333, 333)]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_3_sources() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .combine_latest(observable_1)
        .combine_latest(observable_2);

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
    assert_eq!(checker.values(), [((222, "111"), 0.111)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(
        checker.values(),
        [((222, "111"), 0.111), ((333, "111"), 0.111)]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [
            ((222, "111"), 0.111),
            ((333, "111"), 0.111),
            ((333, "222"), 0.111)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.222).is_continue());
    assert_eq!(
        checker.values(),
        [
            ((222, "111"), 0.111),
            ((333, "111"), 0.111),
            ((333, "222"), 0.111),
            ((333, "222"), 0.222)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            ((222, "111"), 0.111),
            ((333, "111"), 0.111),
            ((333, "222"), 0.111),
            ((333, "222"), 0.222)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            ((222, "111"), 0.111),
            ((333, "111"), 0.111),
            ((333, "222"), 0.111),
            ((333, "222"), 0.222)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            ((222, "111"), 0.111),
            ((333, "111"), 0.111),
            ((333, "222"), 0.111),
            ((333, "222"), 0.222)
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty_1st_source() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (_sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_empty_2nd_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

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
    assert_eq!(checker.values(), [(222, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111"), (444, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.combine_latest(observable_1);

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
    assert_eq!(checker.values(), [(222, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111"), (444, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_source_and_another_source_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().combine_latest(subject.clone());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 111), (222, 222)]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(
        checker.values(),
        [(111, 111), (222, 111), (222, 222), (333, 222), (333, 333)]
    );
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.values(),
        [(111, 111), (222, 111), (222, 222), (333, 222), (333, 333)]
    );
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (channels_1, source_1) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.combine_latest(source_1);
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
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [(222, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(222, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 333).is_continue());
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 444).is_continue());
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.values(),
        [(222, "111"), (333, "111"), (444, "111")]
    );
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels_1.on_next(1, "222").is_continue());
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    channels_1.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [(222, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(
        checker_2.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Completed);
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

    let observable = observable.combine_latest(observable_1);

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
    assert_eq!(checker.values(), [(&value_2, &value_3)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [(&value_2, &value_3)]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.combine_latest(observable_1);

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

    let observable = observable.combine_latest(another_source_observable);
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
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(another_source_channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(another_source_channels.state(1), ChannelState::Subscribed);

    another_source_channels.on_termination(0, Termination::<Infallible>::Completed);
    another_source_channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(another_source_channels.state(0), ChannelState::Completed);
    assert_eq!(checker_2.values(), [(111, "111")]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(another_source_channels.state(1), ChannelState::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable_1.combine_latest(observable_2).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next("111").is_stop());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable
        .combine_latest(observable_1)
        .combine_latest(observable_2)
        .combine_latest(observable_3);

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
    assert_eq!(checker.values(), [(((333, "111"), 0.111), true)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(((333, "111"), 0.111), true), (((333, "222"), 0.111), true)]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(0.222).is_continue());
    assert_eq!(
        checker.values(),
        [
            (((333, "111"), 0.111), true),
            (((333, "222"), 0.111), true),
            (((333, "222"), 0.222), true)
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
            (((333, "111"), 0.111), true),
            (((333, "222"), 0.111), true),
            (((333, "222"), 0.222), true)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (((333, "111"), 0.111), true),
            (((333, "222"), 0.111), true),
            (((333, "222"), 0.222), true)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (((333, "111"), 0.111), true),
            (((333, "222"), 0.111), true),
            (((333, "222"), 0.222), true)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_3.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (((333, "111"), 0.111), true),
            (((333, "222"), 0.111), true),
            (((333, "222"), 0.222), true)
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
}

#[test]
fn test_multiple_operation_same_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut another_source_subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = observable
        .combine_latest(another_source_subject.clone())
        .combine_latest(another_source_subject.clone());

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
    assert_eq!(checker.values(), [(("111", 333), 333)]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(another_source_subject.on_next(444).is_continue());
    assert_eq!(
        checker.values(),
        [
            (("111", 333), 333),
            (("111", 444), 333),
            (("111", 444), 444)
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [
            (("111", 333), 333),
            (("111", 444), 333),
            (("111", 444), 444),
            (("222", 444), 444),
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (("111", 333), 333),
            (("111", 444), 333),
            (("111", 444), 444),
            (("222", 444), 444),
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    another_source_subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (("111", 333), 333),
            (("111", 444), 333),
            (("111", 444), 444),
            (("222", 444), 444),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = CombineLatest::new((observable, observable_1));

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
    assert_eq!(checker.values(), [(222, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker.values(), [(222, "111"), (333, "111"), (444, "111")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [(222, "111"), (333, "111"), (444, "111"), (444, "222")]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    assert!(sender.on_next(555).is_continue());
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (555, "222")
        ]
    );
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            (222, "111"),
            (333, "111"),
            (444, "111"),
            (444, "222"),
            (555, "222")
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut sender_1, source_1, _) = test_channel();
    let source_1 = source_1.start_with([111]);
    let (checker, observer) = Checker::new();

    let observable = source.combine_latest(source_1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert_eq!(checker.values(), vec![(111, 111), (111, 222)]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), vec![(111, 111), (111, 222), (333, 222)]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![(111, 111), (111, 222), (333, 222)]);
    assert_eq!(checker.state(), State::Active);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), vec![(111, 111), (111, 222), (333, 222)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = Empty.combine_latest(observable);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").combine_latest(observable);

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
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.combine_latest(observable_1);

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

    let observable = observable.combine_latest(observable_1);

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

    let observable = observable.combine_latest(observable_1);

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
        let observable = observable.combine_latest(another_source_subject);

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
        let observable = observable.combine_latest(another_source_subject);

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

        let observable = observable.combine_latest(boundary);

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
    let observable = observable.combine_latest(another_source_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel::<'_, String, _>();
    let observable = observable.combine_latest(another_source_observable);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, another_source_observable, _) = test_channel::<'_, (), String>();
    let observable = observable.combine_latest(another_source_observable);

    observable.filter(|_| true);
}

#[test]
fn test_observable_types_without_clone() {
    use rx_rust::observable::ObservableTypes;

    struct NotClone;

    fn inspect_tuple<OE: ObservableTypes<Item = (NotClone, bool)>>(_: OE) {}
    fn inspect_list<OE: ObservableTypes<Item = Vec<NotClone>>>(_: OE) {}

    inspect_tuple(CombineLatest::new((Just::new(NotClone), Just::new(true))));
    inspect_list(CombineLatest::new_from_iter([Just::new(NotClone)]));
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// its latest value still combines with those of the other one. Once the other lets go too,
/// nothing more can come, and the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1
        .combine_latest(observable_2)
        .subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    sender_1.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(10).is_continue());
    assert!(sender_2.on_next(20).is_continue());
    assert_eq!(checker.values(), [(1, 10), (1, 20)]);
    assert_eq!(checker.state(), State::Active);

    sender_2.abandon();
    assert_eq!(checker.values(), [(1, 10), (1, 20)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_2.state(), ChannelState::Abandoned);
}

#[test]
fn test_completed_three_sources() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, &str, Infallible>();
    let (mut sender_3, observable_3, _) = test_channel::<'_, f64, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription =
        CombineLatest::new((observable_1, observable_2, observable_3)).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_2.on_next("a").is_continue());
    assert!(checker.values().is_empty());

    assert!(sender_3.on_next(0.5).is_continue());
    assert!(sender_2.on_next("b").is_continue());
    assert_eq!(checker.values(), [(1, "a", 0.5), (1, "b", 0.5)]);

    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert!(sender_3.on_next(1.5).is_continue());
    sender_3.on_termination(Termination::Completed);
    assert_eq!(checker.values().last(), Some(&(1, "b", 1.5)));
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_eight_sources() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = CombineLatest::new((
        Just::new(1),
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

    let subscription = CombineLatest::new((source(0), source(1), source(2)))
        .subscribe_with_callback(|_| {}, |_| {});
    subscription.dispose();
    // The last subscribed first.
    assert_eq!(order.clone_value(), [2, 1, 0]);
}

#[test]
fn test_error_on_sub_skips_later_sources() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::<_, &str>::new();

    // The first one fails as it is subscribed: the others are not subscribed at all.
    let _subscription = CombineLatest::new((
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
    let (checker, observer) = Checker::new();
    // The filter gives no exact size, and the result exceeds the tuple arity limit.
    let sources = (0..16)
        .filter(|value| value % 2 == 0)
        .map(|value| Just::new(value).into_shared());
    let _subscription = CombineLatest::new_from_iter(sources).subscribe(observer);
    assert_eq!(checker.values(), [vec![0, 2, 4, 6, 8, 10, 12, 14]]);
    assert_eq!(checker.state(), State::Completed);
}

/// The list form, [`CombineLatest::new_from_iter`].
mod from_iter {
    use crate::tests_utils::DURATION_10_MS;
    use crate::tests_utils::checker::State;
    use crate::tests_utils::drop_probe::{DropCount, DropProbe};
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
    use rx_rust::operators::creating::just::Just;
    use rx_rust::operators::creating::throw::Throw;
    use rx_rust::subject::publish_subject::PublishSubject;
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::combining::combine_latest::CombineLatest,
    };
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_completed() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = CombineLatest::new_from_iter(vec![observable; 3]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(channels.len(), 3);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(2, 3).is_continue());
        assert!(checker.values().is_empty());

        // Every source has emitted: the first combination, in the order of the list.
        assert!(channels.on_next(1, 2).is_continue());
        assert_eq!(checker.values(), [vec![1, 2, 3]]);

        assert!(channels.on_next(0, 10).is_continue());
        assert!(channels.on_next(2, 30).is_continue());
        assert_eq!(
            checker.values(),
            [vec![1, 2, 3], vec![10, 2, 3], vec![10, 2, 30]]
        );

        channels.on_termination(0, Termination::Completed);
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.state(), State::Active);
        // A completed source keeps its latest item.
        assert!(channels.on_next(2, 300).is_continue());
        assert_eq!(checker.values().last(), Some(&vec![10, 2, 300]));

        channels.on_termination(2, Termination::Completed);
        assert_eq!(checker.values().len(), 4);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_without_value() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = CombineLatest::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());

        // No combination can ever be made: the stream ends at once.
        channels.on_termination(1, Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_completed_empty() {
        let (checker, observer) = Checker::<Vec<i32>, Infallible>::new();

        let _subscription =
            CombineLatest::new_from_iter(Vec::<Just<i32>>::new()).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_single() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = CombineLatest::new_from_iter([observable]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(0, 2).is_continue());
        channels.on_termination(0, Termination::Completed);
        assert_eq!(checker.values(), [vec![1], vec![2]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_sources_are_same() {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let (checker, observer) = Checker::new();

        let _subscription =
            CombineLatest::new_from_iter(vec![subject.clone(); 2]).subscribe(observer);
        assert!(subject.on_next(1).is_continue());
        assert!(subject.on_next(2).is_continue());
        assert_eq!(checker.values(), [vec![1, 1], vec![2, 1], vec![2, 2]]);

        subject.on_termination(Termination::Completed);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error() {
        let (channels, observable) = test_channels::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let _subscription = CombineLatest::new_from_iter(vec![observable; 3]).subscribe(observer);
        for index in 0..3 {
            assert!(channels.on_next(index, index as i32).is_continue());
        }
        channels.on_termination(1, Termination::Error("error"));
        assert_eq!(checker.values(), [vec![0, 1, 2]]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(channels.state(1), ChannelState::Error("error"));
        assert_eq!(channels.state(2), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_unsubscribe() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let subscription = CombineLatest::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());

        subscription.dispose();
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_unsubscribe_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let sources: Vec<_> = (0..3)
            .map(|index| {
                let order = order.clone();
                Create::shared_boxed(move |_observer: SendBoxedObserver<'_, i32, Infallible>| {
                    let order = order.clone();
                    DisposeOnDrop::new(CallbackDisposal::new(move || {
                        order.lock().unwrap().push(index);
                    }))
                })
            })
            .collect();

        let subscription =
            CombineLatest::new_from_iter(sources).subscribe_with_callback(|_| {}, |_| {});
        subscription.dispose();
        // The last subscribed first.
        assert_eq!(*order.lock().unwrap(), [2, 1, 0]);
    }

    #[test]
    fn test_ref() {
        let values = [111, 222];
        let error = 333;

        let (channels, observable) = test_channels();
        let (checker, observer) = Checker::new();

        let _subscription = CombineLatest::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, &values[0]).is_continue());
        assert!(channels.on_next(1, &values[1]).is_continue());
        channels.on_termination(0, Termination::Error(&error));
        assert_eq!(checker.values(), [vec![&values[0], &values[1]]]);
        assert_eq!(checker.state(), State::Error(&error));
    }

    #[test]
    fn test_async() {
        block_on(|scheduler| async move {
            let (channels, observable) = test_channels::<'_, i32, Infallible>();
            let (checker, observer) = Checker::new();

            let observable = CombineLatest::new_from_iter(vec![observable; 2]);

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
            assert_eq!(channels.state(1), ChannelState::Unsubscribed);
        });
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let (channels, observable) = test_channels::<'_, i32, &str>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let observable_1 = CombineLatest::new_from_iter(vec![observable; 2]);
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert_eq!(channels.len(), 4);

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

        let observable = CombineLatest::new_from_iter(vec![observable; 2]).take(1);

        let _subscription = observable.subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_stop());
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_multiple_operation() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let combined = CombineLatest::new_from_iter(vec![observable; 2])
            .map(|values| values.iter().sum::<i32>());
        let observable = CombineLatest::new_from_iter([combined]);

        let _subscription = observable.subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());
        assert_eq!(checker.values(), [vec![3]]);
    }

    #[test]
    fn test_next_on_sub() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let observable = CombineLatest::new_from_iter([Just::new(1), Just::new(2), Just::new(3)]);

        // Each emits and completes as it is subscribed: the last one completes the combination, and
        // its completion is the last of all.
        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![1, 2, 3]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_next_on_sub_with_start_with() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = CombineLatest::new_from_iter(vec![observable.start_with([0]); 2]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![0, 0]]);

        assert!(channels.on_next(1, 1).is_continue());
        assert_eq!(checker.values(), [vec![0, 0], vec![0, 1]]);
    }

    #[test]
    fn test_complete_on_sub() {
        let (checker, observer) = Checker::<Vec<Infallible>, Infallible>::new();

        let _subscription = CombineLatest::new_from_iter([Empty, Empty]).subscribe(observer);
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
                .into_send_boxed(),
            observable.into_send_boxed(),
        ];

        // The first one fails as it is subscribed: the second is not subscribed at all.
        let _subscription = CombineLatest::new_from_iter(sources).subscribe(observer);
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
                    assert!(observer.on_next(111).is_stop());
                }))
            });

        let subscription = CombineLatest::new_from_iter([observable]).subscribe(observer);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_abandon_after_next() {
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
        let (mut sender_2, observable_2, _channel_checker_2) =
            test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription =
            CombineLatest::new_from_iter([observable_1, observable_2]).subscribe(observer);
        assert!(sender_1.on_next(1).is_continue());
        sender_1.abandon();
        assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
        assert_eq!(checker.state(), State::Active);

        // The other one goes on, combined with the latest item of the one that let go.
        assert!(sender_2.on_next(2).is_continue());
        assert_eq!(checker.values(), [vec![1, 2]]);

        sender_2.abandon();
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_drop_replaced_value_outside_lock() {
        let (channels, observable) = test_channels::<'_, Arc<Mutex<DropProbe>>, Infallible>();
        let (checker, observer) = Checker::new();
        let channels_probe = channels.clone();

        let _subscription = CombineLatest::new_from_iter(vec![observable; 2])
            .map(|values| values.len())
            .subscribe(observer);
        assert!(
            channels
                .on_next(1, Arc::new(Mutex::new(DropProbe::new())))
                .is_continue()
        );

        // The item a newer one replaces is dropped outside the lock: a drop that re-enters the operator
        // would deadlock otherwise, or panic on the re-entrancy check of a debug build.
        let drops = DropCount::new();
        let probe = DropProbe::new()
            .on_drop(drops.callback())
            .on_drop(Box::new(move || {
                assert!(
                    channels_probe
                        .on_next(1, Arc::new(Mutex::new(DropProbe::new())))
                        .is_continue()
                );
            }));
        assert!(
            channels
                .on_next(0, Arc::new(Mutex::new(probe)))
                .is_continue()
        );
        assert_eq!(checker.values(), [2]);
        assert!(
            channels
                .on_next(0, Arc::new(Mutex::new(DropProbe::new())))
                .is_continue()
        );
        // The combinations are gone, so the replaced item was the last owner of the probe. Its drop
        // sent a new item, which made one more combination.
        assert_eq!(drops.get(), 1);
        assert_eq!(checker.values(), [2, 2, 2]);
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

            let observable = CombineLatest::new_from_iter([observable]);

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
            let observable = CombineLatest::new_from_iter([observable]);

            let (_, mut observer) = Checker::<_, Infallible>::new();
            assert!(observer.on_next(vec![Some(&life_marker_2)]).is_continue());
            let _subscription = observable.subscribe(observer);
        }
    }

    #[test]
    fn test_clone() {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::Error(TestStruct));
            DisposeOnDrop::default()
        });
        let observable = CombineLatest::new_from_iter(vec![observable]);
        _ = observable.clone(); // Make sure it's Clone when E is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        let observable = CombineLatest::new_from_iter([Just::new(1)]);

        let observable = CombineLatest::new_from_iter([observable]);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        let observable = CombineLatest::new_from_iter([Just::new(1)]);

        CombineLatest::new_from_iter([observable]);
    }
}
