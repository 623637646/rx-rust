mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::shared_sender::SharedSender;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::connectable::connectable_controller::ConnectableController;
use rx_rust::operators::creating::defer::Defer;
use rx_rust::utils::mutable::Mutable;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::utils::types::Shared;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    subject::async_subject::AsyncSubject,
};
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_error() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Error("error")
    );
}

#[test]
fn test_unsubscribe() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    subscription_2.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    drop(controller);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Unsubscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_ref() {
    let counter = AtomicUsize::new(0);
    let value_1 = 111;
    let value_2 = 222;

    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| {
            let counter = counter.fetch_add(1, Ordering::SeqCst) + 1;
            match counter {
                1 => &value_1,
                2 => &value_2,
                _ => panic!(),
            }
        })
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [&value_2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let counter = Shared::new(AtomicUsize::new(0));
        let sender = SharedSender::default();
        let channel_checker = Shared::new(Mutable::new(None));
        let sender_cloned = sender.clone();
        let channel_checker_cloned = channel_checker.clone();
        let observable = Defer::new(move || {
            let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
            assert!(sender_cloned.set(sender));
            assert!(
                channel_checker_cloned
                    .replace_value(Some(channel_checker))
                    .is_none()
            );
            observable
        });
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let controller = observable
            .map(move |_| counter.fetch_add(1, Ordering::SeqCst) + 1)
            .publish_last();
        let observable = controller.observable();
        let observable_1 = observable.clone();
        let observable_2 = observable.clone();

        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(channel_checker.with_ref(Option::is_none));

        let _subscription_1 = runtime
            .spawn(async move { observable_1.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(channel_checker.with_ref(Option::is_none));

        let _controller = runtime
            .spawn(async move { controller.connect() })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
            ChannelState::Subscribed
        );

        let sender = runtime
            .spawn(async move {
                sender.on_next(());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), []);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
            ChannelState::Subscribed
        );

        let _subscription_2 = runtime
            .spawn(async move { observable_2.subscribe(observer_2) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), []);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
            ChannelState::Subscribed
        );

        let sender = runtime
            .spawn(async move {
                sender.on_next(());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), []);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), []);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(
            channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
            ChannelState::Subscribed
        );

        runtime
            .spawn(async move { sender.on_termination(Termination::<Infallible>::Completed) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [2]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Completed);
        assert_eq!(
            channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
            ChannelState::Completed
        );
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let controller = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish_last();
    let observable = controller.observable();
    let observable_1 = observable.clone().take(1);
    let observable_2 = observable.clone().take(2);

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_multiple_operation() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1);
    let controller_1 = observable.publish_last();
    let controller_2 = controller_1.observable().publish_last();
    let observable = controller_2.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller_1 = controller_1.connect();
    let _controller_2 = controller_2.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_without_convenient_api() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1);
    let controller = ConnectableController::<_, AsyncSubject<'_, _, _>>::new(
        observable,
        AsyncSubject::default(),
    );
    let observable = controller.observable();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let _controller = controller.connect();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
    );
}

#[test]
fn test_share_api() {
    let counter = AtomicUsize::new(0);
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .share_last();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(channel_checker.with_ref(Option::is_none));

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_next(());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Subscribed
    );

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(
        channel_checker.with_ref(|checker| checker.as_ref().unwrap().state()),
        ChannelState::Completed
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
        let controller = observable.publish_last();
        let observable = controller.observable();
        _subscription_1 = controller.connect();

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
        let controller = observable.publish_last();
        let observable = controller.observable();

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
        let controller = observable.publish_last();
        let observable = controller.observable();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let controller = observable.publish_last();
    let observable = controller.observable();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let sender = SharedSender::default();
    let channel_checker = Shared::new(Mutable::new(None));
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = Defer::new(move || {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        assert!(sender_cloned.set(sender));
        assert!(
            channel_checker_cloned
                .replace_value(Some(channel_checker))
                .is_none()
        );
        observable
    });
    let controller = observable.publish_last();
    let observable = controller.observable();

    observable.filter(|_| true);
}
