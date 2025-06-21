mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{
        combining::merge::Merge,
        creating::{create::Create, just::Just, throw::Throw},
    },
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_initialized());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_2);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());
}

#[test]
fn test_completed_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_initialized());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_2);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Just<i32>, _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.values().is_empty());
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_completed_same_inner() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut subject_1 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(subject_1.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subject_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(subject_1.clone());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subject_1.on_next(222);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());

    subject_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_completed_new_from_iter() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Merge::new_from_iter([observable_1, observable_2]);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());
}

#[test]
fn test_error_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_initialized());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_2);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_error("error"));
}

#[test]
fn test_error_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_initialized());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_2);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());
}

#[test]
fn test_error_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, Throw<_>, _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_error_same_inner() {
    let (mut sender, observable, channel_checker) = test_channel();
    let mut subject_1 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(subject_1.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subject_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(subject_1.clone());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subject_1.on_next(222);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());

    subject_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_completed());

    subject_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222, 222, 333, 333]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_completed());
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.merge();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_initialized());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_1);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_initialized());

    sender.on_next(observable_2);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    sender_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());

    subscription.unsubscribe();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
    assert!(channel_checker_1.is_unsubscribed());
    assert!(channel_checker_2.is_unsubscribed());
}

#[test]
fn test_unsubscribe_with_publish_subject() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject_1.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_next(subject_2.clone());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject_2.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subject_1.on_next(333);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert!(checker_2.is_active());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert!(checker_2.is_active());

    subject_2.on_next(444);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert!(checker_2.is_active());

    subject_2.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let value_4 = 444;
    let error = -1;

    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject_1.on_next(&value_1);
    assert_eq!(checker.values(), [&value_1]);
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert_eq!(checker.values(), [&value_1]);
    assert!(checker.is_active());

    subject_2.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert!(checker.is_active());

    subject_1.on_next(&value_3);
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3]);
    assert!(checker.is_active());

    subject_2.on_next(&value_4);
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3, &value_4]);
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2, &value_3, &value_4]);
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;
    let mut error = -1;

    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(Just::new(&mut value_1).map_infallible_to_error());
        observer.on_next(Just::new(&mut value_2).map_infallible_to_error());
        observer.on_next(Just::new(&mut value_3).map_infallible_to_error());
        observer.on_termination(Termination::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let observable = observable.merge();

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => *error *= 2,
        },
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
    assert_eq!(error, -2);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let subject_1 = PublishSubject::default();
    let subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let subject_1_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(subject_1_cloned.clone());
    });
    handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let mut subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let subject_2_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(subject_2_cloned.clone());
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    let mut subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let mut subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(333);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Completed);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let mut subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(444);
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());

    let subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject_1.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_next(subject_2.clone());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject_2.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subject_1.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert!(checker_2.is_active());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert!(checker_2.is_active());

    subject_2.on_next(444);
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert!(checker_2.is_active());

    subject_2.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let mut subject_3 = PublishSubject::default();
    let mut subject_4 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge().merge();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject_1.on_next(subject_3.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject_2.on_next(subject_4.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject_3.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject_4.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject_3.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject_4.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Merge::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert!(checker.is_completed());
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
            observer.on_next(Just::new(1));
            observer.on_termination(Termination::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.merge();

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
        let observable = Create::new(
            |observer: BoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_1 = Some(observer);
                Subscription::new_none_disposal()
            },
        );
        let observable = observable.merge();

        let (_, mut observer) = Checker::new();
        observer.on_next(&life_marker_2);
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
        let observable = Create::new(
            |observer: BoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_or = Some(observer);
                Subscription::new_with_disposal_callback(|| {
                    life_marker_sub.consume_ref();
                })
            },
        );

        let observable = observable.merge();

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(Just::new(TestStruct).map_infallible_to_error());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.merge();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, _> = PublishSubject::default();
    let observable = subject.merge();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, Infallible> = PublishSubject::default();
    let observable = subject.merge();

    observable.filter(|_| true);
}
