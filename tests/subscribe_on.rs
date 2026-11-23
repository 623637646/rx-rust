mod tests_utils;

use crate::tests_utils::{
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel, test_channels},
    test_scheduler::block_on,
    test_struct::TestStruct,
    thread_checker_scheduler::{CallThreads, ThreadCheckerExecutor, check_threads},
};
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::SendBoxedObserver},
    operators::{
        creating::{create::Create, empty::Empty, never::Never, throw::Throw},
        utility::subscribe_on::SubscribeOn,
    },
    scheduler::virtual_time::VirtualTime,
};
use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[test]
fn test_completed() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
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
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        thread_1.run_until_stalled().await;
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
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (channels, observable) = test_channels();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable.clone(),
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        thread_1.run_until_stalled().await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
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
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler())
        .take(1);

        let _subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_break());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let thread_2 = ThreadCheckerExecutor::new("thread_2");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history_a = Arc::new(AtomicUsize::new(0));
        let call_history_b = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history_a,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        );
        let observable = check_threads!(
            observable.subscribe_on(thread_1.scheduler()),
            call_history_b,
            CallThreads {
                subscription: Some("thread_2"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_2.scheduler());

        let subscription = observable.subscribe(observer);
        // The outer `subscribe_on` subscribes the inner one, which then subscribes the source.
        thread_2.run_until_stalled().await;
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00000011);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history_a.load(Ordering::SeqCst), 0b00001111);
        assert_eq!(call_history_b.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = SubscribeOn::new(
            check_threads!(
                observable,
                call_history,
                CallThreads {
                    subscription: Some("thread_1"),
                    next: None,
                    termination: None,
                    disposal: None
                }
            ),
            thread_1.scheduler(),
        );

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        assert!(sender.on_next(444).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        assert!(sender.on_next(111).is_continue());
        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b11001111);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
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
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
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
fn test_order_with_continuous_next() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            observable,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let _subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00000011);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(sender.on_next(*i).is_continue());
        }
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00111111);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (sender, source, _) = test_channel();
        let source = source.start_with([111]);
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            source,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: Some("thread_1"),
                termination: None,
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

        let subscription = observable.subscribe(observer);
        thread_1.run_until_stalled().await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(call_history.load(Ordering::SeqCst), 0b00001111);

        sender.on_termination(Termination::<Infallible>::Completed);
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
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            Empty,
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: Some("thread_1"),
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

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
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let (checker, observer) = Checker::new();
        let call_history = Arc::new(AtomicUsize::new(0));

        let observable = check_threads!(
            Throw::new("error"),
            call_history,
            CallThreads {
                subscription: Some("thread_1"),
                next: None,
                termination: Some("thread_1"),
                disposal: None
            }
        )
        .subscribe_on(thread_1.scheduler());

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
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. SubscribeOn hands the observer to its source, so the value still reaches
    // it.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_continue());
        }))
    });

    let observable = observable.subscribe_on(scheduler.clone());

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. SubscribeOn hands the observer to its source, so the completion still
    // reaches it.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.subscribe_on(scheduler.clone());

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_unsub() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. SubscribeOn hands the observer to its source, so the error still reaches
    // it.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.subscribe_on(scheduler.clone());

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_clone() {
    block_on(|_| async move {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(TestStruct).is_continue());
            observer.on_termination(Termination::Error(TestStruct));
            DisposeOnDrop::default()
        });
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let observable = observable.subscribe_on(thread_1.scheduler());
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let observable = Never.subscribe_on(thread_1.scheduler());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|_| async move {
        let thread_1 = ThreadCheckerExecutor::new("thread_1");
        let observable = Never.subscribe_on(thread_1.scheduler());

        observable.filter(|_| true);
    });
}
