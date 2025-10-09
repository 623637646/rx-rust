#![cfg(not(feature = "single-threaded"))]
mod tests_utils;

use crate::tests_utils::DURATION_5_MS;
use crate::tests_utils::test_scheduler::get_thread_name;
use crate::tests_utils::{
    checker::State,
    test_channel::{ChannelState, test_channel},
    test_runtime::block_on,
};
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    disposable::{Disposable, subscription::Subscription},
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, never::Never},
        utility::subscribe_on::SubscribeOn,
    },
    scheduler::Scheduler,
    subject::publish_subject::PublishSubject,
    utils::types::Shared,
};
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

#[test]
fn test_unsubscribe_immediately() {
    block_on(|runtime| async move {
        let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        subscription.dispose();
        check_with_spawned_and_abort_late!(
            runtime,
            {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Dropped);
                match channel_checker.state() {
                    ChannelState::Initialized => {
                        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000000)
                    }
                    ChannelState::Subscribed => unreachable!(),
                    ChannelState::Completed => unreachable!(),
                    ChannelState::Error(_) => unreachable!(),
                    ChannelState::Unsubscribed => {
                        assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011)
                    }
                }
                assert_eq!(runtime.get_alive_tasks_count(), 0);
            },
            {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Dropped);
                assert_eq!(channel_checker.state(), ChannelState::Initialized);
                assert_eq!(call_history.load(Ordering::SeqCst), 0b00000000);
                assert_eq!(runtime.get_alive_tasks_count(), 0);
            }
        );
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

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
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender.on_next(333);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        runtime
            .spawn(async move {
                sender.on_next(444);
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        runtime
            .spawn(async move { subscription.dispose() })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
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
        let observable = subject
            .clone()
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        subject.on_next(111);
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        subject.on_next(222);
        subject.on_next(333);
        assert_eq!(checker_1.values(), [111, 222, 333]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111, 222, 333]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        subject.on_next(444);
        subject.on_termination(Termination::<Infallible>::Completed);
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
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"))
            .take(1);

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history_a = Shared::new(AtomicUsize::new(0));
        let call_history_a_1 = call_history_a.clone();
        let call_history_a_2 = call_history_a.clone();
        let call_history_a_3 = call_history_a.clone();
        let call_history_a_4 = call_history_a.clone();
        let call_history_a_5 = call_history_a.clone();
        let call_history_a_6 = call_history_a.clone();
        let call_history_a_7 = call_history_a.clone();
        let call_history_a_8 = call_history_a.clone();
        let call_history_b = Shared::new(AtomicUsize::new(0));
        let call_history_b_1 = call_history_b.clone();
        let call_history_b_2 = call_history_b.clone();
        let call_history_b_3 = call_history_b.clone();
        let call_history_b_4 = call_history_b.clone();
        let call_history_b_5 = call_history_b.clone();
        let call_history_b_6 = call_history_b.clone();
        let call_history_b_7 = call_history_b.clone();
        let call_history_b_8 = call_history_b.clone();

        // Custom operations
        let observable = observable
            .do_before_subscription(move || {
                call_history_a_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_a_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_a_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_a_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_a_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_a_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_a_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_a_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"))
            .do_before_subscription(move || {
                call_history_b_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_after_subscription(move || {
                call_history_b_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_before_next(move |_| {
                call_history_b_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_b_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_b_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_b_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_b_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_b_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_2"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
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
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
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
        let observable = SubscribeOn::new(
            observable
                .do_before_subscription(move || {
                    call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), Some("thread_1"));
                })
                .do_after_subscription(move || {
                    call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), Some("thread_1"));
                })
                .do_before_next(move |_| {
                    call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                })
                .do_after_next(move |_| {
                    call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                })
                .do_before_termination(move |_| {
                    call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                })
                .do_after_termination(move |_| {
                    call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                })
                .do_before_disposal(move || {
                    call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                })
                .do_after_disposal(move || {
                    call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                    assert_eq!(get_thread_name(), None);
                }),
            runtime.clone_with_thread_name("thread_1"),
        );

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_termination(Termination::<Infallible>::Completed);
        subscription.dispose();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.subscribe_on(runtime.clone_with_thread_name("thread_1"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        check_with_spawned_late!(
            runtime,
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Initialized);
                assert_eq!(runtime.get_alive_tasks_count(), 1);
            },
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);
                assert_eq!(runtime.get_alive_tasks_count(), 0);
            }
        );

        sender.on_termination(Termination::<Infallible>::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.subscribe_on(runtime.clone_with_thread_name("thread_1"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        check_with_spawned_late!(
            runtime,
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Initialized);
                assert_eq!(runtime.get_alive_tasks_count(), 1);
            },
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);
                assert_eq!(runtime.get_alive_tasks_count(), 0);
            }
        );

        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_unsub() {
    block_on(|runtime| async move {
        let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.subscribe_on(runtime.clone_with_thread_name("thread_1"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let subscription = observable.subscribe(observer);
        check_with_spawned_late!(
            runtime,
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Initialized);
                assert_eq!(runtime.get_alive_tasks_count(), 1);

                subscription.dispose();
                assert_eq!(runtime.get_alive_tasks_count(), 0);

                runtime.sleep(DURATION_5_MS).await;
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Dropped);
                assert_eq!(channel_checker.state(), ChannelState::Initialized);
                assert_eq!(runtime.get_alive_tasks_count(), 0);
            },
            {}
        );
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|runtime| async move {
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
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            sender.on_next(*i);
        }
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
    });
}

#[test]
fn test_immediate_next() {
    block_on(|runtime| async move {
        let subject = BehaviorSubject::new(111);
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
        let observable = subject
            .clone()
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        subject.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_immediate_completed() {
    block_on(|runtime| async move {
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
        let observable = Empty
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00110011);

        subscription.dispose();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
    });
}

#[test]
fn test_immediate_error() {
    block_on(|runtime| async move {
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
        let observable = Throw::new("error")
            .do_before_subscription(move || {
                call_history_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_subscription(move || {
                call_history_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_next(move |_| {
                call_history_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_next(move |_| {
                call_history_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_termination(move |_| {
                call_history_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00110011);

        subscription.dispose();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11110011);
    });
}

#[test]
fn test_observe_on_with_subscribe_on() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history_a = Shared::new(AtomicUsize::new(0));
        let call_history_a_1 = call_history_a.clone();
        let call_history_a_2 = call_history_a.clone();
        let call_history_a_3 = call_history_a.clone();
        let call_history_a_4 = call_history_a.clone();
        let call_history_a_5 = call_history_a.clone();
        let call_history_a_6 = call_history_a.clone();
        let call_history_a_7 = call_history_a.clone();
        let call_history_a_8 = call_history_a.clone();
        let call_history_b = Shared::new(AtomicUsize::new(0));
        let call_history_b_1 = call_history_b.clone();
        let call_history_b_2 = call_history_b.clone();
        let call_history_b_3 = call_history_b.clone();
        let call_history_b_4 = call_history_b.clone();
        let call_history_b_5 = call_history_b.clone();
        let call_history_b_6 = call_history_b.clone();
        let call_history_b_7 = call_history_b.clone();
        let call_history_b_8 = call_history_b.clone();

        // Custom operations
        let observable = observable
            .observe_on(runtime.clone_with_thread_name("thread_o_1"))
            .do_before_subscription(move || {
                call_history_a_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_s_1"));
            })
            .do_after_subscription(move || {
                call_history_a_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_s_1"));
            })
            .do_before_next(move |_| {
                call_history_a_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_1"));
            })
            .do_after_next(move |_| {
                call_history_a_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_1"));
            })
            .do_before_termination(move |_| {
                call_history_a_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_1"));
            })
            .do_after_termination(move |_| {
                call_history_a_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_1"));
            })
            .do_before_disposal(move || {
                call_history_a_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_a_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .observe_on(runtime.clone_with_thread_name("thread_o_2"))
            .subscribe_on(runtime.clone_with_thread_name("thread_s_1"))
            .do_before_subscription(move || {
                call_history_b_1.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_s_2"));
            })
            .do_after_subscription(move || {
                call_history_b_2.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_s_2"));
            })
            .do_before_next(move |_| {
                call_history_b_3.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_2"));
            })
            .do_after_next(move |_| {
                call_history_b_4.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_2"));
            })
            .do_before_termination(move |_| {
                call_history_b_5.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_2"));
            })
            .do_after_termination(move |_| {
                call_history_b_6.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_o_2"));
            })
            .do_before_disposal(move || {
                call_history_b_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_b_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .subscribe_on(runtime.clone_with_thread_name("thread_s_2"));

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_5_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

        sender.on_next(111);
        runtime.sleep(DURATION_5_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(222);
        sender.on_next(333);
        runtime.sleep(DURATION_5_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        runtime.sleep(DURATION_5_MS).await;
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
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.subscribe_on(runtime.clone_with_thread_name("thread_1"));
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.subscribe_on(runtime.clone_with_thread_name("thread_1"));

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.subscribe_on(runtime.clone_with_thread_name("thread_1"));

        observable.filter(|_| true);
    });
}
