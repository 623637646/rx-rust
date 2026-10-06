mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::thread_checker_scheduler::{ThreadCheckerScheduler, get_thread_name};
use crate::tests_utils::{
    checker::State,
    test_channel::{ChannelState, test_channel},
    test_scheduler::block_on,
};
use futures::executor::ThreadPool;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::runtime::futures::ThreadPoolScheduler;
use rx_rust::{
    disposable::Disposable,
    observable::Subscription,
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::never::Never, utility::observe_on::ObserveOn},
};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

/// How long a race condition test waits for the pool to finish a round: far beyond what a round
/// takes, it only turns a hang into a failure.
const RACE_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn test_completed() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_termination(Termination::Error("error"));
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription.dispose();
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
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
fn test_async() {
    block_on(|scheduler| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();
        let call_history_5 = call_history.clone();
        let call_history_6 = call_history.clone();
        let call_history_7 = call_history.clone();
        let call_history_8 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(move || {
                call_history_5.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(move || {
                call_history_6.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await;
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        scheduler
            .spawn(async move {
                assert!(sender.on_next(444).is_continue());
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await;
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        scheduler.spawn(async move { subscription.dispose() }).await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (channels, observable) = test_channels();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .clone()
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert!(channels.on_next(0, 333).is_continue());
        assert!(channels.on_next(1, 333).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker_1.values(), [111, 222, 333]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [111, 222, 333]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(channels.on_next(0, 444).is_continue());
        assert!(channels.on_next(1, 444).is_continue());
        channels.on_termination(0, Termination::<Infallible>::Completed);
        channels.on_termination(1, Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
        assert_eq!(checker_1.values(), [111, 222, 333, 444]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Completed);
        assert_eq!(checker_2.values(), [111, 222, 333, 444]);
        assert_eq!(checker_2.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);

        subscription_1.dispose();
        subscription_2.dispose();
        assert_eq!(checker_1.values(), [111, 222, 333, 444]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Completed);
        assert_eq!(checker_2.values(), [111, 222, 333, 444]);
        assert_eq!(checker_2.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11111111);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();
        let call_history_5 = call_history.clone();
        let call_history_6 = call_history.clone();
        let call_history_7 = call_history.clone();
        let call_history_8 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(move || {
                call_history_5.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(move || {
                call_history_6.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(move || {
                call_history_7.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_disposal(move || {
                call_history_8.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

/// Values queued before the scheduler task runs are delivered as one batch. Disposing partway
/// through that batch must stop it: the values behind the disposal are never observed.
#[test]
fn test_unsub_in_the_middle_of_a_batch() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let delivered = Arc::new(AtomicUsize::new(0));
        let delivered_1 = delivered.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_after_next(move |_| {
                delivered_1.fetch_add(1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        // All three values are queued before the scheduler task runs, so `observe_on` has them
        // buffered as a single batch by the time it starts delivering.
        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert!(checker.values().is_empty());
        thread_1.run_until_stalled().await;

        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        // `take(1)` disposes while `observe_on` is delivering the batch, so 222 and 333 stay
        // buffered instead of being pushed downstream.
        assert_eq!(delivered.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let thread_2 = ThreadCheckerScheduler::new("thread_2");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history_a = Arc::new(AtomicUsize::new(0));
        let call_history_a_1 = call_history_a.clone();
        let call_history_a_2 = call_history_a.clone();
        let call_history_a_3 = call_history_a.clone();
        let call_history_a_4 = call_history_a.clone();
        let call_history_b = Arc::new(AtomicUsize::new(0));
        let call_history_b_1 = call_history_b.clone();
        let call_history_b_2 = call_history_b.clone();
        let call_history_b_3 = call_history_b.clone();
        let call_history_b_4 = call_history_b.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history_a.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history_a.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_a_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_a_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_a_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_a_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history_a.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history_a.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .observe_on(thread_2.clone())
            .do_before_subscription(|| {
                call_history_b.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history_b.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_b_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_after_next(move |_| {
                call_history_b_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_before_termination(move |_| {
                call_history_b_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_after_termination(move |_| {
                call_history_b_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_2"));
            })
            .do_before_disposal(|| {
                call_history_b.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history_b.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
        thread_2.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        thread_1.run_until_stalled().await;
        thread_2.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
        thread_2.run_until_stalled().await;
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = ObserveOn::new(observable, thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Error("error"));
        // The error is queued behind 111 before the task runs, and an error preempts the values
        // buffered before it.
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00110011);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        // Disposed while 111 is still queued: the task must not deliver it.
        subscription.dispose();
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_termination(Termination::<Infallible>::Completed);
        // Disposed while the termination is still queued: the task must not deliver it.
        subscription.dispose();
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        sender.on_termination(Termination::Error("error"));
        // Disposed while the termination is still queued: the task must not deliver it.
        subscription.dispose();
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11000011);
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = observable
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(sender.on_next(*i).is_continue());
        }
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (sender, source, _) = test_channel();
        let source = source.start_with([111]);
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = source
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_termination(Termination::<Infallible>::Completed);
        thread_1.run_until_stalled().await;
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
fn test_complete_on_sub() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = Empty
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
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
fn test_error_on_sub() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerScheduler::new("thread_1");
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));
        let call_history_1 = call_history.clone();
        let call_history_2 = call_history.clone();
        let call_history_3 = call_history.clone();
        let call_history_4 = call_history.clone();

        let observable = Throw::new("error")
            .observe_on(thread_1.clone())
            .do_before_subscription(|| {
                call_history.fetch_or(1 << 0, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_subscription(|| {
                call_history.fetch_or(1 << 1, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_before_next(move |_| {
                call_history_1.fetch_or(1 << 2, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_next(move |_| {
                call_history_2.fetch_or(1 << 3, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_termination(move |_| {
                call_history_3.fetch_or(1 << 4, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_after_termination(move |_| {
                call_history_4.fetch_or(1 << 5, Ordering::SeqCst);
                assert_eq!(get_thread_name(), Some("thread_1"));
            })
            .do_before_disposal(|| {
                call_history.fetch_or(1 << 6, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            })
            .do_after_disposal(|| {
                call_history.fetch_or(1 << 7, Ordering::SeqCst);
                assert_eq!(get_thread_name(), None);
            });

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
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
fn test_next_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source emits from inside its own disposal, so the value arrives while downstream is
        // unsubscribing. It must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_stop());
                }))
            });

        let observable = observable.observe_on(scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source completes from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The termination must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        let observable = observable.observe_on(scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_error_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source fails from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The error must be dropped instead of reaching the observer.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        });

        let observable = observable.observe_on(scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

/// The other tests decide when the scheduler's thread runs. Here a thread pool delivers while the
/// test thread is still sending, so that the task stopping and a value being queued at that moment
/// race: whatever the interleaving, every value reaches the observer once and in order, then the
/// completion.
#[test]
fn test_race_condition_between_next_and_delivery() {
    const VALUES: i32 = 100;
    let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());

    for _ in 0..500 {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let (terminated, termination) = mpsc::channel();

        let observable = observable
            .observe_on(scheduler.clone())
            .do_after_termination(move |_| terminated.send(()).unwrap());

        let _subscription = observable.subscribe(observer);
        for value in 0..VALUES {
            assert!(sender.on_next(value).is_continue());
        }
        sender.on_termination(Termination::Completed);

        termination
            .recv_timeout(RACE_TIMEOUT)
            .expect("the completion is delivered");
        assert_eq!(checker.values(), (0..VALUES).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }
}

/// `take` ends the stream on a pool thread while the test thread is still sending. `Stop` is a
/// guarantee: a send answered with it comes after the taken values were delivered, and nothing
/// sent afterwards, or racing the disposal, reaches the observer.
#[test]
fn test_race_condition_unsub_on_next_by_take() {
    const TAKEN: usize = 3;
    let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());

    for _ in 0..500 {
        let mut sender = None;
        let (disposed, disposal) = mpsc::channel();
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, usize, Infallible>| {
                sender = Some(observer);
                Subscription::new(CallbackDisposal::new(move || disposed.send(()).unwrap()))
            });
        let (checker, observer) = Checker::new();
        let (terminated, termination) = mpsc::channel();

        let observable = observable
            .observe_on(scheduler.clone())
            .take(TAKEN)
            .do_after_termination(move |_| terminated.send(()).unwrap());

        let _subscription = observable.subscribe(observer);
        let mut sender = sender.unwrap();
        // Whether a send is answered `Stop`, and which one, depends on the interleaving.
        if let Some(stopped_at) = (0..100).find(|value| sender.on_next(*value).is_stop()) {
            assert!(stopped_at >= TAKEN - 1);
            assert_eq!(checker.values(), (0..TAKEN).collect::<Vec<_>>());
        }
        // `sender` stays alive until the end: a source that drops its observer abandons the
        // subscription, and the values still queued with it.

        termination
            .recv_timeout(RACE_TIMEOUT)
            .expect("take completes");
        disposal
            .recv_timeout(RACE_TIMEOUT)
            .expect("take disposes the source");
        assert_eq!(checker.values(), (0..TAKEN).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed);
    }
}

#[test]
fn test_observe_on_with_subscribe_on() {
    block_on(|_| async move {
        let thread_o_1 = ThreadCheckerScheduler::new("thread_o_1");
        let thread_o_2 = ThreadCheckerScheduler::new("thread_o_2");
        let thread_s_1 = ThreadCheckerScheduler::new("thread_s_1");
        let thread_s_2 = ThreadCheckerScheduler::new("thread_s_2");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history_a = Arc::new(AtomicUsize::new(0));
        let call_history_a_1 = call_history_a.clone();
        let call_history_a_2 = call_history_a.clone();
        let call_history_a_3 = call_history_a.clone();
        let call_history_a_4 = call_history_a.clone();
        let call_history_a_5 = call_history_a.clone();
        let call_history_a_6 = call_history_a.clone();
        let call_history_a_7 = call_history_a.clone();
        let call_history_a_8 = call_history_a.clone();
        let call_history_b = Arc::new(AtomicUsize::new(0));
        let call_history_b_1 = call_history_b.clone();
        let call_history_b_2 = call_history_b.clone();
        let call_history_b_3 = call_history_b.clone();
        let call_history_b_4 = call_history_b.clone();
        let call_history_b_5 = call_history_b.clone();
        let call_history_b_6 = call_history_b.clone();
        let call_history_b_7 = call_history_b.clone();
        let call_history_b_8 = call_history_b.clone();

        let observable = observable
            .observe_on(thread_o_1.clone())
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
            .observe_on(thread_o_2.clone())
            .subscribe_on(thread_s_1.clone())
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
            .subscribe_on(thread_s_2.clone());

        let subscription = observable.subscribe(observer);
        // The outer `subscribe_on` subscribes the inner one, which then subscribes the source.
        thread_s_2.run_until_stalled().await;
        thread_s_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        thread_o_1.run_until_stalled().await;
        thread_o_2.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        thread_o_1.run_until_stalled().await;
        thread_o_2.run_until_stalled().await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        thread_o_1.run_until_stalled().await;
        thread_o_2.run_until_stalled().await;
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
fn test_lifetime_sub() {
    block_on(|_| async move {
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
                Subscription::new(CallbackDisposal::new(|| {
                    life_marker.consume_ref();
                }))
            });

            let observable = observable.observe_on(ThreadCheckerScheduler::new("thread_1"));

            let (_, observer) = Checker::new();
            _subscription = observable.subscribe(observer);
        }
    });
}

#[test]
fn test_clone() {
    block_on(|_| async move {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(TestStruct).is_continue());
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.observe_on(ThreadCheckerScheduler::new("thread_1"));
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|_| async move {
        let observable = Never.observe_on(ThreadCheckerScheduler::new("thread_1"));

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|_| async move {
        let observable = Never.observe_on(ThreadCheckerScheduler::new("thread_1"));

        observable.filter(|_| true);
    });
}
