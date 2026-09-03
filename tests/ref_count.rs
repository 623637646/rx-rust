mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::connectable::ref_count::RefCount;
use rx_rust::operators::creating::defer::Defer;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::safe_lock_option;
use rx_rust::safe_lock_option_disposable;
use rx_rust::safe_lock_option_observer;
use rx_rust::safe_lock_vec;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::Subject;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, MutableBool, MutableBoolHelper, MutableHelper, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
}

#[test]
fn test_error() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_unsubscribe() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2, 3]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2, 3]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_last_unsubscribe_removes_subject_observer_before_disconnecting_source() {
    struct RecordSourceStateOnDrop {
        source_disconnected: Shared<MutableBool>,
        disconnected_when_dropped: Shared<MutableBool>,
    }

    impl Observer<(), Infallible> for RecordSourceStateOnDrop {
        fn on_next(&mut self, _: ()) {}

        fn on_termination(self, _: Termination<Infallible>) {}
    }

    impl Drop for RecordSourceStateOnDrop {
        fn drop(&mut self) {
            self.disconnected_when_dropped
                .write(self.source_disconnected.read());
        }
    }

    let source_disconnected = Shared::new(MutableBool::new(false));
    let disconnected_when_observer_dropped = Shared::new(MutableBool::new(false));
    let source_disconnected_cloned = source_disconnected.clone();
    let source = Create::new(move |_: BoxedObserver<'_, (), Infallible>| {
        let source_disconnected = source_disconnected_cloned.clone();
        Subscription::new(CallbackDisposal::new(move || {
            source_disconnected.write(true);
        }))
    });
    let observable = source.publish().ref_count();
    let observer = RecordSourceStateOnDrop {
        source_disconnected: source_disconnected.clone(),
        disconnected_when_dropped: disconnected_when_observer_dropped.clone(),
    };

    let subscription = observable.subscribe(observer);
    assert!(!source_disconnected.read());
    assert!(!disconnected_when_observer_dropped.read());

    subscription.dispose();

    assert!(source_disconnected.read());
    assert!(!disconnected_when_observer_dropped.read());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            let counter = counter.fetch_add(1, Ordering::SeqCst) + 1;
            match counter {
                1 => &value_1,
                2 => &value_2,
                _ => panic!(),
            }
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Error(&error));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error(&error)
    );
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let counter = Shared::new(AtomicUsize::new(0));
        let sender = Shared::new(Mutable::new(None));
        let channel_checker = Shared::new(Mutable::new(None));
        let sender_cloned = sender.clone();
        let channel_checker_cloned = channel_checker.clone();
        let observable = Defer::new(move || {
            let (sender, observable, channel_checker) = test_channel();
            assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
            assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
            observable
        });
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = observable
            .map(move |_| counter.fetch_add(1, Ordering::SeqCst) + 1)
            .publish()
            .ref_count();
        let observable_1 = observable.clone();
        let observable_2 = observable.clone();

        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(channel_checker.test_lock_ref().is_none());

        let subscription_1 = runtime
            .spawn(async move { observable_1.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );

        let sender = runtime
            .spawn(async move {
                safe_lock_option_observer!(on_next: sender, ());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );

        let _subscription_2 = runtime
            .spawn(async move { observable_2.subscribe(observer_2) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );

        let sender = runtime
            .spawn(async move {
                safe_lock_option_observer!(on_next: sender, ());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );

        runtime
            .spawn(async move { subscription_1.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );

        runtime
            .spawn(async move {
                safe_lock_option_observer!(on_termination: sender, Termination::Error("error"))
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Error("error"));
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Error("error")
        );
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count()
        .take(1);
    let observable_1 = observable.clone();

    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription = observable_1.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_multiple_operation() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count()
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_without_convenient_api() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish();
    let observable = RefCount::new(observable);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_complete_on_next() {
    let counter = AtomicUsize::new(0);
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_next() {
    let counter = AtomicUsize::new(0);
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .clone()
                .on_termination(Termination::Error("error"));
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Error("error"));

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    // no unsubscribe
    let subscription = observable_1.subscribe(observer_1);

    // unsubscribe before on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_next(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_next(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription.dispose();
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_sub_on_next() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                // subscribe before on_next
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_next(value);
                // subscribe after on_next
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [2]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
}

#[test]
fn test_next_on_next() {
    let counter = AtomicUsize::new(0);
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            // The subject is terminated as soon as `on_termination` is called, which the callback
            // of the first value did: the callback of the second sees it terminated while the
            // termination is still queued behind that value.
            assert_eq!(subject_cloned.terminated().is_some(), value > 1);
            if value < 3 {
                subject_cloned.on_next(());
            }
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(());
    // The events are delivered in the order they were sent: the third value is sent from the
    // callback of the second one, after that callback has already sent the termination, so it
    // arrives once the subject has terminated and is dropped.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_on_completed() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
}

#[test]
fn test_sub_on_completed() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
}

#[test]
fn test_unsub_on_error() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_sub_on_error() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_next_on_sub() {
    let counter = AtomicUsize::new(0);
    let mut subject = BehaviorSubject::new(());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject
        .clone()
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(());
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [3]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [3]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [3]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let counter = AtomicUsize::new(0);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Empty
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let counter = AtomicUsize::new(0);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Throw::new("error")
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_sub_on_sub() {
    let counter = AtomicUsize::new(0);
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_1 = observable_1
        .hook_on_subscription(|observable, observer| {
            let sub = observable.subscribe(observer);
            let sub2 = observable_2.subscribe(observer_2);
            assert!(safe_lock_option!(replace: subscription_2, sub2).is_none());
            sub
        })
        .subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_disposable!(dispose: subscription_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_subscribe_after_all_unsubscribed() {
    let counter = AtomicUsize::new(0);
    let sender_channel_checker = Shared::new(Mutable::new(Vec::new()));
    let sender_channel_checker_cloned = sender_channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        safe_lock_vec!(push: sender_channel_checker_cloned, (sender, channel_checker));
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    sender_channel_checker.lock_mut(|mut lock| {
        lock[0].0.on_next(());
    });
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    sender_channel_checker.lock_mut(|mut lock| {
        lock[0].0.on_next(());
    });
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 1);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Unsubscribed
    );

    let subscription_3 = observable_3.subscribe(observer_3);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 2);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .last()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    sender_channel_checker.lock_mut(|mut lock| {
        lock[1].0.on_next(());
    });
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 2);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .last()
            .unwrap()
            .1
            .state(),
        ChannelState::Subscribed
    );

    subscription_3.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 2);
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .first()
            .unwrap()
            .1
            .state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(
        sender_channel_checker
            .test_lock_ref()
            .last()
            .unwrap()
            .1
            .state(),
        ChannelState::Unsubscribed
    );
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
            observer.on_next(111);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.publish().ref_count();

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let life_marker = Shared::new(Mutable::new(None));

    // Error
    // let life_marker = Shared::new(Mutable::new(None));
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            safe_lock_option!(replace: life_marker, Some(observer));
            Subscription::default()
        });
        let observable = observable.publish().ref_count();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_2]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker);

        let observable =
            Create::new(|_: BoxedObserver<'_, &TestStruct, Infallible>| Subscription::default());
        let observable = observable.publish().ref_count();
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
    let observable = observable.publish().ref_count();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let observable = observable.publish().ref_count();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let observable = observable.publish().ref_count();

    observable.filter(|_| true);
}
