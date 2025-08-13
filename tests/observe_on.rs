#![cfg(not(feature = "single-threaded"))]
mod tests_utils;

use crate::tests_utils::{
    checker::State,
    test_channel::{ChannelState, test_channel},
    test_runtime::block_on,
    test_thread_scheduler::{TestThreadScheduler, get_thread_name},
};
use rx_rust::{
    disposable::{Disposable, subscription::Subscription},
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, never::Never},
        utility::observe_on::ObserveOn,
    },
    scheduler::Scheduler,
    subject::publish_subject::PublishSubject,
    utils::types::Shared,
};
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(222);
    sender.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(222);
    sender.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_termination(Termination::Error("error"));
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
}

#[test]
fn test_async() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();
    let call_history_5 = call_history.clone();
    let call_history_6 = call_history.clone();
    let call_history_7 = call_history.clone();
    let call_history_8 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(move || {
            call_history_5.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(move || {
            call_history_6.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(move || {
            call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(move || {
            call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = std::thread::spawn(move || observable.subscribe(observer))
        .join()
        .unwrap();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    let mut sender = std::thread::spawn(move || {
        sender.on_next(111);
        sender
    })
    .join()
    .unwrap();
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    let mut sender = std::thread::spawn(move || {
        sender.on_next(222);
        sender.on_next(333);
        sender
    })
    .join()
    .unwrap();
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    std::thread::spawn(move || {
        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
    })
    .join()
    .unwrap();
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    std::thread::spawn(move || subscription.dispose())
        .join()
        .unwrap();

    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = subject
        .clone()
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    subject.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    subject.on_next(222);
    subject.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    subject.on_next(444);
    subject.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    subscription_1.dispose();
    subscription_2.dispose();
    assert_eq!(checker_1.values(), [111, 222, 333, 444]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222, 333, 444]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();
    let call_history_5 = call_history.clone();
    let call_history_6 = call_history.clone();
    let call_history_7 = call_history.clone();
    let call_history_8 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(move || {
            call_history_5.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(move || {
            call_history_6.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(move || {
            call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_disposal(move || {
            call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history_a = Shared::new(AtomicUsize::new(0));
    let call_history_a_1 = call_history_a.clone();
    let call_history_a_2 = call_history_a.clone();
    let call_history_a_3 = call_history_a.clone();
    let call_history_a_4 = call_history_a.clone();
    let call_history_b = Shared::new(AtomicUsize::new(0));
    let call_history_b_1 = call_history_b.clone();
    let call_history_b_2 = call_history_b.clone();
    let call_history_b_3 = call_history_b.clone();
    let call_history_b_4 = call_history_b.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history_a.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history_a.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_a_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_a_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_a_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_a_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history_a.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history_a.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .observe_on(TestThreadScheduler::new("thread_2"))
        .do_before_subscription(|| {
            call_history_b.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history_b.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_b_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_2");
        })
        .do_after_next(move |_| {
            call_history_b_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_2");
        })
        .do_before_termination(move |_| {
            call_history_b_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_2");
        })
        .do_after_termination(move |_| {
            call_history_b_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_2");
        })
        .do_before_disposal(|| {
            call_history_b.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history_b.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
    assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
    assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(222);
    sender.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
    assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00111111);
    assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00111111);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history_a.load(Ordering::SeqCst), 0b11111111);
    assert_eq!(call_history_b.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = ObserveOn::new(observable, TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(222);
    sender.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_complete_after_next() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
}

#[test]
fn test_error_after_next() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    sender.on_termination(Termination::Error("error"));
    std::thread::sleep(Duration::from_millis(10));
    if checker.values().is_empty() {
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00110011);
    } else if checker.values() == [111] {
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
    } else {
        panic!();
    }
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsub_after_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    subscription.dispose();
    std::thread::sleep(Duration::from_millis(10));
    if checker.values().is_empty() {
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
    } else if checker.values() == [111] {
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    } else {
        panic!();
    }
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_after_completed() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_termination(Termination::<Infallible>::Completed);
    subscription.dispose();
    std::thread::sleep(Duration::from_millis(10));
    match checker.state() {
        State::Dropped => {
            assert_eq!(checker.values(), []);
            assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
        }
        State::Completed => {
            assert_eq!(checker.values(), []);
            assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
        }
        State::Active => panic!(),
        State::Error(_) => panic!(),
    }
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsub_after_error() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_termination(Termination::Error("error"));
    subscription.dispose();
    std::thread::sleep(Duration::from_millis(10));
    match checker.state() {
        State::Dropped => {
            assert_eq!(checker.values(), []);
            assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
        }
        State::Error(_) => {
            assert_eq!(checker.values(), []);
            assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
        }
        State::Active => panic!(),
        State::Completed => panic!(),
    }
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_undisposed_schedule() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    sender.on_next(111);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(222);
    sender.on_next(333);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
}

#[test]
fn test_scheduler_should_be_disposed_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.observe_on(runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        runtime.sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.observe_on(runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        runtime.sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_unsub() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.observe_on(runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        sender.on_next(111);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        subscription.dispose();
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        runtime.sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty() || checker.values() == [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_order_with_continuous_next() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let call_history = Shared::new(AtomicUsize::new(0));
    let call_history_1 = call_history.clone();
    let call_history_2 = call_history.clone();
    let call_history_3 = call_history.clone();
    let call_history_4 = call_history.clone();

    // Custom operations
    let observable = observable
        .observe_on(TestThreadScheduler::new("thread_1"))
        .do_before_subscription(|| {
            call_history.fetch_or(1 << 0, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_subscription(|| {
            call_history.fetch_or(1 << 1, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_before_next(move |_| {
            call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_next(move |_| {
            call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_termination(move |_| {
            call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_after_termination(move |_| {
            call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "thread_1");
        })
        .do_before_disposal(|| {
            call_history.fetch_or(1 << 6, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        })
        .do_after_disposal(|| {
            call_history.fetch_or(1 << 7, Ordering::SeqCst);
            assert_eq!(get_thread_name(), "");
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

    let values = (0..100000).collect::<Vec<_>>();
    for i in &values {
        sender.on_next(*i);
    }
    sender.on_termination(Termination::<Infallible>::Completed);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(checker.values(), values);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
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
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.observe_on(TestThreadScheduler::new("thread_1"));

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.observe_on(TestThreadScheduler::new("thread_1"));
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Never.observe_on(TestThreadScheduler::new("thread_1"));

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Never.observe_on(TestThreadScheduler::new("thread_1"));

    observable.filter(|_| true);
}
