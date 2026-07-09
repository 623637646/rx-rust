mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::test_struct::TestStruct;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::backpressure::on_backpressure_latest::OnBackpressureLatest;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    subject::publish_subject::PublishSubject,
};
use rx_rust::{safe_lock, safe_lock_option, safe_lock_vec};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed_no_request() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_completed_request_before_emit() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_completed_request_after_emit() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_completed_request_before_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_completed_request_after_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_error_no_request() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_error_request_before_emit() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_error_request_after_emit() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_some());
}

#[test]
fn test_error_request_before_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_error_request_after_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let request_callback = Shared::new(Mutable::new(Vec::new()));
    let request_callback_cloned = request_callback.clone();
    let observable = subject.clone();
    let observable = observable
        .on_backpressure_latest()
        .map(move |(value, request)| {
            safe_lock_vec!(push: request_callback_cloned, request);
            value
        });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 0);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    safe_lock!(mem_take: request_callback)
        .into_iter()
        .for_each(|request| request.request());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 1);

    safe_lock!(mem_take: request_callback)
        .into_iter()
        .for_each(|request| request.request());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(request_callback.test_lock_ref().len(), 0);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(&value_1);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(&value_3);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [&value_1, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [&value_1, &value_3]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert!(request_callback.test_lock_ref().is_none());
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
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
    let observable = observable.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |termination| assert!(matches!(termination, Termination::Error("error"))),
        );
    assert!(request_callback.test_lock_ref().is_some());
    drop(request_callback);
    subscription.dispose();

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 222);
    assert_eq!(value_3, 333);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.on_backpressure_latest();

        let request_callback = Shared::new(Mutable::new(None));
        let request_callback_cloned = request_callback.clone();
        let _subscription = runtime
            .spawn(async move {
                observable
                    .map(move |(value, request)| {
                        safe_lock_option!(replace: request_callback_cloned, request);
                        value
                    })
                    .subscribe(observer)
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(request_callback.test_lock_ref().is_none());

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(request_callback.test_lock_ref().is_some());

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(request_callback.test_lock_ref().is_some());

        let sender = runtime
            .spawn(async move {
                sender.on_next(333);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(request_callback.test_lock_ref().is_some());

        let request_callback_cloned = request_callback.clone();
        runtime
            .spawn(async move {
                safe_lock_option!(take: request_callback_cloned)
                    .unwrap()
                    .request();
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(request_callback.test_lock_ref().is_some());

        runtime
            .spawn(async move {
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert!(request_callback.test_lock_ref().is_some());

        let request_callback_cloned = request_callback.clone();
        runtime
            .spawn(async move {
                safe_lock_option!(take: request_callback_cloned)
                    .unwrap()
                    .request();
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 333]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert!(request_callback.test_lock_ref().is_none());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let request_callback = Shared::new(Mutable::new(Vec::new()));
    let request_callback_cloned = request_callback.clone();
    let observable = subject.clone();
    let observable = observable
        .on_backpressure_latest()
        .map(move |(value, request)| {
            safe_lock_vec!(push: request_callback_cloned, request);
            value
        });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 0);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    safe_lock!(mem_take: request_callback)
        .into_iter()
        .for_each(|request| request.request());
    assert_eq!(checker_1.values(), [111, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(request_callback.test_lock_ref().len(), 2);

    safe_lock!(mem_take: request_callback)
        .into_iter()
        .for_each(|request| request.request());
    assert_eq!(checker_1.values(), [111, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(request_callback.test_lock_ref().len(), 0);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<_, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.on_backpressure_latest().take(1);

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let request_callback_1 = Shared::new(Mutable::new(None));
    let request_callback_1_cloned = request_callback_1.clone();
    let request_callback_2 = Shared::new(Mutable::new(None));
    let request_callback_2_cloned = request_callback_2.clone();
    let observable = observable
        .on_backpressure_latest()
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_1_cloned, request);
            value
        })
        .on_backpressure_latest()
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_2_cloned, request);
            value
        });
    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_none());
    assert!(request_callback_2.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback_1)
        .unwrap()
        .request();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback_2)
        .unwrap()
        .request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback_1.test_lock_ref().is_some());
    assert!(request_callback_2.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback_1)
        .unwrap()
        .request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback_1.test_lock_ref().is_none());
    assert!(request_callback_2.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback_2)
        .unwrap()
        .request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback_1.test_lock_ref().is_none());
    assert!(request_callback_2.test_lock_ref().is_none());
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = OnBackpressureLatest::new(observable);

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_none());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(request_callback.test_lock_ref().is_some());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_next_on_sub() {
    let mut subject = BehaviorSubject::<'_, _, Infallible>::new(111);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(request_callback.test_lock_ref().is_some());

    subject.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(request_callback.test_lock_ref().is_some());

    subject.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert!(request_callback.test_lock_ref().is_some());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Active);
    assert!(request_callback.test_lock_ref().is_some());

    safe_lock_option!(take: request_callback).unwrap().request();
    assert_eq!(checker.values(), [111, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Empty.on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert!(request_callback.test_lock_ref().is_none());
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Throw::new("error").on_backpressure_latest();

    let request_callback = Shared::new(Mutable::new(None));
    let request_callback_cloned = request_callback.clone();
    let _subscription = observable
        .map(move |(value, request)| {
            safe_lock_option!(replace: request_callback_cloned, request);
            value
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert!(request_callback.test_lock_ref().is_none());
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
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.on_backpressure_latest();

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
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable
            .on_backpressure_latest()
            .map(move |(value, _)| value);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
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
    let observable = observable.on_backpressure_latest();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.on_backpressure_latest();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.on_backpressure_latest();

    observable.filter(|_| true);
}
