mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::connectable::connectable_observable::ConnectableObservable;
use rx_rust::operators::creating::defer::Defer;
use rx_rust::utils::types::{Mutable, MutableHelper, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    subject::publish_subject::PublishSubject,
};
use rx_rust::{safe_lock_option, safe_lock_option_observer, safe_lock_vec};
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
        .publish();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_2 = observable.connect().unwrap();
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

    let _subscription = observable_2.subscribe(observer_2);
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

    safe_lock_option_observer!(on_termination: sender, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Completed);
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
        .publish();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription = observable.connect().unwrap();
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

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
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
        .publish();
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
    assert!(channel_checker.test_lock_ref().is_none());

    let subscription = observable.connect().unwrap();
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

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    subscription.dispose();
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_ref() {
    let counter = AtomicUsize::new(0);
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

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
        .publish();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription = observable.connect().unwrap();
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

    safe_lock_option_observer!(on_termination: sender, Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Error(&error));
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
            .publish();
        let observable_1 = observable.clone();
        let observable_2 = observable.clone();

        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(channel_checker.test_lock_ref().is_none());

        let _subscription_1 = runtime
            .spawn(async move { observable_1.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(channel_checker.test_lock_ref().is_none());

        let _subscription = runtime
            .spawn(async move { observable.connect() })
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
            .spawn(async move {
                safe_lock_option_observer!(on_termination: sender, Termination::Error("error"))
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Error("error"));
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
        .publish();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription = observable.connect().unwrap();
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

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
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
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish();
    let observable_1 = observable.clone().take(1);
    let observable_2 = observable.clone().take(2);

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_2 = observable.connect().unwrap();
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
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    safe_lock_option_observer!(on_next: sender, ());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2, 3]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
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
    let observable = observable.map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1);
    let co_1 = observable.publish();
    let co_2 = co_1.clone().publish();
    let observable_1 = co_2.clone();
    let observable_2 = co_2.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription = co_1.connect().unwrap();
    let _subscription = co_2.connect().unwrap();
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

    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
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
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(safe_lock_option!(replace: sender_cloned, sender).is_none());
        assert!(safe_lock_option!(replace: channel_checker_cloned, channel_checker).is_none());
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1);
    let observable = ConnectableObservable::<_, PublishSubject<'_, _, _>>::new(
        observable,
        PublishSubject::default(),
    );
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.test_lock_ref().is_none());

    let _subscription_2 = observable.connect().unwrap();
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

    let _subscription = observable_2.subscribe(observer_2);
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

    safe_lock_option_observer!(on_termination: sender, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
}

#[test]
fn test_share_api() {
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
        .share();
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
fn test_connect_without_subscription() {
    let counter = AtomicUsize::new(0);
    let sender_channel_checker = Shared::new(Mutable::new(Vec::new()));
    let sender_channel_checker_cloned = sender_channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        safe_lock_vec!(push: sender_channel_checker_cloned, (sender, channel_checker));
        observable
    });

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish();

    assert_eq!(sender_channel_checker.test_lock_ref().len(), 0);

    // connect without subscription
    let connect_subscription = observable.connect().unwrap();
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

    // unsub
    connect_subscription.dispose();
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
}

#[test]
fn test_invalid_connect_and_reconnect() {
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
        .publish();
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
    assert_eq!(sender_channel_checker.test_lock_ref().len(), 0);

    let connect_subscription_1 = observable.clone().connect().unwrap();
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

    let connect_subscription_2 = observable.clone().connect();
    assert!(connect_subscription_2.is_none());
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
        ChannelState::Subscribed
    );

    connect_subscription_1.dispose();
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

    let connect_subscription_3 = observable.clone().connect().unwrap();
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
        ChannelState::Subscribed
    );

    connect_subscription_3.dispose();
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
    let _subscription_1;

    // Error
    // let _subscription_1;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.publish();
        _subscription_1 = observable.clone().connect().unwrap();

        let (_, observer) = Checker::<_, ()>::new();
        let _subscription_2 = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker = None;

    // Error
    // let mut life_marker = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker = Some(observer);
            Subscription::default()
        });
        let observable = observable.publish();

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
        let observable = observable.publish();
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
    let observable = observable.publish();
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
    let observable = observable.publish();

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
    let observable = observable.publish();

    observable.filter(|_| true);
}
