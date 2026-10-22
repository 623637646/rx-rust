mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::connectable::connectable_controller::ConnectableController;
use rx_rust::operators::connectable::ref_count::RefCount;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::Subject;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::subject::replay_subject::ReplaySubject;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::{MutableBoolHelper, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Flow, Observer, Termination},
};
use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
}

#[test]
fn test_error() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2, 3]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2, 3]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_last_unsubscribe_removes_subject_observer_before_disconnecting_source() {
    struct RecordSourceStateOnDrop {
        source_disconnected: Arc<AtomicBool>,
        disconnected_when_dropped: Arc<AtomicBool>,
    }

    impl Observer<(), Infallible> for RecordSourceStateOnDrop {
        fn on_next(&mut self, _: ()) -> Flow {
            Flow::Continue
        }

        fn on_termination(self, _: Termination<Infallible>) {}
    }

    impl Drop for RecordSourceStateOnDrop {
        fn drop(&mut self) {
            self.disconnected_when_dropped
                .write(self.source_disconnected.read());
        }
    }

    let source_disconnected = Arc::new(AtomicBool::new(false));
    let disconnected_when_observer_dropped = Arc::new(AtomicBool::new(false));
    let source_disconnected_cloned = source_disconnected.clone();
    let source = Create::shared_boxed(move |_: SendBoxedObserver<'_, (), Infallible>| {
        let source_disconnected = source_disconnected_cloned.clone();
        DisposeOnDrop::new(CallbackDisposal::new(move || {
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
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [&value_2]);
    assert_eq!(checker_2.state(), State::Error(&error));
    assert_eq!(channels.state(0), ChannelState::Error(&error));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let counter = Arc::new(AtomicUsize::new(0));
        let (channels, observable) = test_channels();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

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
        assert_eq!(channels.len(), 0);

        let subscription_1 = scheduler
            .spawn(async move { observable_1.subscribe(observer_1) })
            .await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(0, ()).is_continue());
            })
            .await;
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let _subscription_2 = scheduler
            .spawn(async move { observable_2.subscribe(observer_2) })
            .await;
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(0, ()).is_continue());
            })
            .await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        scheduler
            .spawn(async move { subscription_1.dispose() })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move { channels_cloned.on_termination(0, Termination::Error("error")) })
            .await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [2]);
        assert_eq!(checker_2.state(), State::Error("error"));
        assert_eq!(channels.state(0), ChannelState::Error("error"));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count()
        .take(1);
    let observable_1 = observable.clone();

    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.len(), 0);

    let _subscription = observable_1.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_stop());
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_without_convenient_api() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_complete_on_next() {
    let counter = AtomicUsize::new(0);
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

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

    assert!(subject.on_next(()).is_continue());
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
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

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

    assert!(subject.on_next(()).is_continue());
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Error("error"));

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    // no unsubscribe
    let subscription = observable_1.subscribe(observer_1);

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_2
            .hook_on_next(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_next(value)
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_3
            .hook_on_next(move |observer, value| {
                assert!(observer.on_next(value).is_continue());
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                Flow::Continue
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_sub_on_next() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                // subscribe before on_next
                if let Some(observer) = observer_2.take() {
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                assert!(observer.on_next(value).is_continue());
                // subscribe after on_next
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                Flow::Continue
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [2]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
}

#[test]
fn test_next_on_next() {
    let counter = AtomicUsize::new(0);
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();

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
            assert_eq!(subject_cloned.termination().is_some(), value > 1);
            if value < 3 {
                // Like the termination above: the send of `1` is queued behind the running
                // delivery, the send of `2` finds the subject terminated.
                assert_eq!(subject_cloned.on_next(()).is_stop(), value > 1);
            }
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(()).is_continue());
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
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before termination
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_2
            .hook_on_termination(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
}

#[test]
fn test_sub_on_completed() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
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
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    // The subject is reset before the termination is delivered, so both late subscribers begin
    // the next round, whether they subscribe before or after observer 1 is terminated.
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [3]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
}

#[test]
fn test_unsub_on_error() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before termination
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_2
            .hook_on_termination(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_sub_on_error() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable
        .map(|_| counter.fetch_add(1, Ordering::SeqCst) + 1)
        .publish()
        .ref_count();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
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
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    // The subject is reset before the termination is delivered, so both late subscribers begin
    // the next round, whether they subscribe before or after observer 1 is terminated.
    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [3]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
}

#[test]
fn test_next_on_sub() {
    let counter = AtomicUsize::new(0);
    let mut subject = BehaviorSubject::shared(());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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

    assert!(subject.on_next(()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(()).is_continue());
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
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_1 = observable_1
        .hook_on_subscription(|observable, observer| {
            let sub = observable.subscribe(observer);
            let sub2 = observable_2.subscribe(observer_2);
            assert!(subscription_2.replace_value(Some(sub2)).is_none());
            sub
        })
        .subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    if let Some(subscription) = subscription_2.take_value() {
        Disposable::dispose(subscription);
    }
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_unsub() {
    let (checker_1, observer_1) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while the last
    // subscription is being disposed, which disconnects the connectable observable. It must be
    // dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_continue());
        }))
    });

    let observable = observable.publish().ref_count();

    let subscription_1 = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);

    subscription_1.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let (checker_1, observer_1) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while the last
    // subscription is being disposed, which disconnects the connectable observable. The
    // termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.publish().ref_count();

    let subscription_1 = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);

    subscription_1.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
}

#[test]
fn test_error_on_unsub() {
    let (checker_1, observer_1) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while the last subscription
    // is being disposed, which disconnects the connectable observable. The error must be dropped
    // instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.publish().ref_count();

    let subscription_1 = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);

    subscription_1.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
}

#[test]
fn test_sub_on_unsub() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.publish().ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    // The only observer subscribes another one while it is being released, so that subscription
    // runs from inside the disposal that would disconnect the connectable observable.
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_2_cloned = subscription_2.clone();
    let probe = DropProbe::new().on_drop(Box::new(move || {
        subscription_2_cloned.replace_value(Some(observable_2.subscribe(observer_2)));
    }));
    let (mut next_1, termination_1) = observer_1.into_callbacks();

    let subscription_1 = observable_1.subscribe_with_callback(
        move |value| {
            let _ = &probe;
            next_1(value);
        },
        termination_1,
    );
    assert_eq!(channels.len(), 1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert!(subscription_2.with_ref(Option::is_some));
    // The subscription made from inside the disposal arrives before the connection is released,
    // so the connectable observable stays connected to the source it already had.
    assert_eq!(channels.len(), 1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    // That source is the one feeding the second subscriber.
    assert!(channels.on_next(0, 111).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
}

#[test]
fn test_subscribe_after_all_unsubscribed() {
    let counter = AtomicUsize::new(0);
    let (channels, observable) = test_channels::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

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
    assert_eq!(channels.len(), 0);

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);

    let subscription_3 = observable_3.subscribe(observer_3);
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, ()).is_continue());
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    subscription_3.dispose();
    assert_eq!(checker_1.values(), [1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [3]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_subscribe_after_completed() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.publish().ref_count();

    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.len(), 1);

    // The first subscription is still held, but its round ended with the source, so the next
    // subscriber starts over with a new subject and a new connection.
    let subscription_2 = observable.clone().subscribe(observer_2);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    // The first round's subscription leaves without counting out a subscriber of the second.
    drop(subscription_1);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 2).is_continue());
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_2.values(), [2]);
    assert_eq!(checker_2.state(), State::Active);

    drop(subscription_2);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_subscribe_after_error() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.publish().ref_count();

    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());
    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Error("error"));

    let subscription_2 = observable.clone().subscribe(observer_2);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    drop(subscription_1);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 2).is_continue());
    assert_eq!(checker_2.values(), [2]);

    drop(subscription_2);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_reset_on_complete_false() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable.replay(None).reset_on_complete(false).ref_count();

    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(checker_1.state(), State::Completed);

    // The completed subject is kept: a late subscriber gets what it replays, and the source is not
    // connected again.
    let subscription_2 = observable.clone().subscribe(observer_2);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.len(), 1);

    // Not even once every subscriber has left.
    drop(subscription_1);
    drop(subscription_2);
    let _subscription_3 = observable.clone().subscribe(observer_3);
    assert_eq!(checker_3.values(), [1]);
    assert_eq!(checker_3.state(), State::Completed);
    assert_eq!(channels.len(), 1);
}

#[test]
fn test_reset_on_error_false() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.replay(None).reset_on_error(false).ref_count();

    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());
    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.state(), State::Error("error"));

    drop(subscription_1);
    let _subscription_2 = observable.clone().subscribe(observer_2);
    assert_eq!(checker_2.values(), [1]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.len(), 1);
}

#[test]
fn test_reset_on_complete_false_still_resets_on_ref_count_zero() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.replay(None).reset_on_complete(false).ref_count();

    // The last subscriber leaves before the source terminates, so the subject is reset.
    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());
    drop(subscription_1);
    assert_eq!(checker_1.values(), [1]);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);

    let _subscription_2 = observable.clone().subscribe(observer_2);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
}

#[test]
fn test_reset_on_ref_count_zero_false() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = observable
        .replay(None)
        .ref_count()
        .reset_on_ref_count_zero(false);

    let subscription_1 = observable.clone().subscribe(observer_1);
    assert!(channels.on_next(0, 1).is_continue());

    // The source stays connected with no subscriber, and the subject keeps receiving.
    drop(subscription_1);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(channels.on_next(0, 2).is_continue());
    assert_eq!(checker_1.values(), [1]);

    let _subscription_2 = observable.clone().subscribe(observer_2);
    assert_eq!(checker_2.values(), [1, 2]);
    assert_eq!(channels.len(), 1);

    // A termination still ends the round, so the next subscriber starts over.
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker_2.state(), State::Completed);
    let _subscription_3 = observable.clone().subscribe(observer_3);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channels.len(), 2);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
}

#[test]
fn test_race_condition() {
    // Subscribers of a source that completes as soon as it is connected come and go from several
    // threads: every one of them is terminated, and the source is never connected twice at once.
    use std::sync::Barrier;

    const SUBSCRIBERS: usize = 4;
    const ROUNDS: usize = 50;

    for _ in 0..100 {
        let connected = Arc::new(AtomicUsize::new(0));
        let overlapped = Arc::new(AtomicBool::new(false));
        let connected_of_source = connected.clone();
        let overlapped_of_source = overlapped.clone();
        let observable = Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'static, i32, Infallible>| {
                if connected_of_source.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlapped_of_source.write(true);
                }
                let _ = observer.on_next(1);
                observer.on_termination(Termination::Completed);
                let connected = connected_of_source.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    connected.fetch_sub(1, Ordering::SeqCst);
                }))
            },
        )
        .share();
        let barrier = Arc::new(Barrier::new(SUBSCRIBERS));

        let handles: Vec<_> = (0..SUBSCRIBERS)
            .map(|_| {
                let observable = observable.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    // The subscriptions are returned, so that each round's subscribers stay counted
                    // while the other threads start new ones.
                    (0..ROUNDS)
                        .map(|_| {
                            let (checker, observer) = Checker::new();
                            (checker, observable.clone().subscribe(observer))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();
        for (checker, _subscription) in &results {
            // A subscriber that joined a round after its source completed only gets the
            // completion; either way, none is left waiting for a connection.
            let values = checker.values();
            assert!(values == [1] || values.is_empty(), "{values:?}");
            assert_eq!(checker.state(), State::Completed);
        }
        assert!(!overlapped.read(), "the source was connected twice at once");
        drop(results);
        assert_eq!(connected.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn test_race_condition_with_unsubscribe() {
    // Subscribers of a source that never terminates come and go from several threads, so the
    // source is connected and disconnected over and over: never twice at once, and not at all
    // once they are gone.
    use std::sync::Barrier;

    const SUBSCRIBERS: usize = 4;
    const ROUNDS: usize = 50;

    for _ in 0..100 {
        let connected = Arc::new(AtomicUsize::new(0));
        let overlapped = Arc::new(AtomicBool::new(false));
        let connected_of_source = connected.clone();
        let overlapped_of_source = overlapped.clone();
        let observable = Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'static, i32, Infallible>| {
                if connected_of_source.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlapped_of_source.write(true);
                }
                let _ = observer.on_next(1);
                let connected = connected_of_source.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    connected.fetch_sub(1, Ordering::SeqCst);
                }))
            },
        )
        .share_behavior(0);
        let barrier = Arc::new(Barrier::new(SUBSCRIBERS));

        let handles: Vec<_> = (0..SUBSCRIBERS)
            .map(|_| {
                let observable = observable.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..ROUNDS {
                        // What the observer receives is the subject's business: a subscription
                        // made while another thread delivers is served by that thread.
                        let (_, observer) = Checker::new();
                        drop(observable.clone().subscribe(observer));
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(!overlapped.read(), "the source was connected twice at once");
        assert_eq!(connected.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn test_finished_subscriber_releases_the_source() {
    for termination in [Termination::Completed, Termination::Error("error")] {
        let source_dropped = DropCount::new();
        let context_dropped = DropCount::new();
        let source_probe = Arc::new(Mutex::new(source_dropped.probe()));
        let termination_of_source = termination.clone();
        let source = Create::shared_boxed(move |mut observer: SendBoxedObserver<'_, i32, &str>| {
            let _ = &source_probe;
            assert!(observer.on_next(111).is_continue());
            observer.on_termination(termination_of_source);
            DisposeOnDrop::default()
        });
        let observable = ConnectableController::new(source, context_dropped.probe(), |_| {
            ReplaySubject::shared(None)
        })
        .reset_on_complete(false)
        .reset_on_error(false)
        .ref_count();

        let (_, observer) = Checker::new();
        let subscription = observable.clone().subscribe(observer);
        drop(subscription);
        assert_eq!(source_dropped.get(), 0);
        assert_eq!(context_dropped.get(), 0);

        // This subscriber only replays the kept subject: it must not keep the source or the
        // factory's context alive once the last observable handle is consumed.
        let (checker, observer) = Checker::new();
        let subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [111]);
        assert_eq!(
            checker.state(),
            match termination {
                Termination::Completed => State::Completed,
                Termination::Error(error) => State::Error(error),
            }
        );
        assert_eq!(source_dropped.get(), 1);
        assert_eq!(context_dropped.get(), 1);
        drop(subscription);
    }
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
            assert!(observer.on_next(111).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
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
    let life_marker = Arc::new(Mutex::new(None));

    // Error
    // let life_marker = Arc::new(Mutex::new(None));
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker.replace_value(Some(Some(observer)));
            DisposeOnDrop::default()
        });
        let observable = observable.publish().ref_count();

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(vec![&life_marker_2]).is_continue());
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
        assert!(observer.on_next(&life_marker).is_continue());

        let observable =
            Create::shared_boxed(|_: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                DisposeOnDrop::default()
            });
        let observable = observable.publish().ref_count();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer: SendBoxedObserver<'_, i32, &str>| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        DisposeOnDrop::default()
    });
    let observable = observable.publish().ref_count();
    _ = observable.clone();
}

/// The state is behind the subject's mode, not the source's: a `Local` source that is `Send`,
/// multicast through a `Shared` subject, makes a `RefCount` that can be erased into a `Send` box.
#[test]
fn test_send_boxed_local_source() {
    let (checker, observer) = Checker::new();

    let observable = ConnectableController::new(Just::new(111), (), |_| {
        PublishSubject::<'_, i32, Infallible, _>::shared()
    })
    .ref_count()
    .into_send_boxed();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable) = test_channels::<'_, i32, Infallible>();
    let observable = observable.publish().ref_count();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable) = test_channels::<'_, i32, Infallible>();
    let observable = observable.publish().ref_count();

    observable.filter(|_| true);
}

/// Panics from the first value it receives, which a subscriber of a replaying subject receives
/// inside `subscribe`.
#[cfg(panic = "unwind")]
struct PanicOnNext(Option<crate::tests_utils::panic::PanicOnDrop>);

#[cfg(panic = "unwind")]
impl Observer<i32, Infallible> for PanicOnNext {
    fn on_next(&mut self, _value: i32) -> Flow {
        drop(self.0.take());
        Flow::Continue
    }

    fn on_termination(self, _termination: Termination<Infallible>) {}
}

/// Emits `1` as soon as it is connected and stays connected, counting its connections and
/// recording its disconnection.
#[cfg(panic = "unwind")]
macro_rules! connection_counting_source {
    ($connections:expr, $is_disconnected:expr) => {{
        let connections: Arc<AtomicUsize> = $connections;
        let is_disconnected: Arc<AtomicBool> = $is_disconnected;
        Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
                connections.fetch_add(1, Ordering::SeqCst);
                // A subject an observer panicked in is dead, and stops its source.
                let _ = observer.on_next(1);
                let is_disconnected = is_disconnected.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    is_disconnected.write(true);
                }))
            },
        )
    }};
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_late_subscriber_keeps_the_ref_count() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let observable = connection_counting_source!(connections.clone(), is_disconnected.clone())
        .replay(None)
        .ref_count();

    let (checker, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(checker.values(), [1]);
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    let observable_of_panic = observable.clone();
    expect_panic_on_drop(|panic_on_drop| {
        // The subject replays `1` inside `subscribe`, while this subscriber is counted in.
        let _subscription = observable_of_panic.subscribe(PanicOnNext(Some(panic_on_drop)));
    });
    assert!(!is_disconnected.read());

    // The subscriber that never came back is counted out again, so the last live subscription is
    // still the one that disconnects the source.
    drop(subscription);
    assert!(is_disconnected.read());
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    // Held until here, so that the disconnection above is the ref count's own, and not the state
    // being dropped with its last handle.
    drop(observable);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_first_subscriber_gives_the_controller_back() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let observable = connection_counting_source!(connections.clone(), is_disconnected.clone())
        .publish_behavior(0)
        .ref_count();

    let observable_of_panic = observable.clone();
    expect_panic_on_drop(|panic_on_drop| {
        // The subject hands `0` over inside `subscribe`, while this first subscriber holds the
        // controller it was about to connect.
        let _subscription = observable_of_panic.subscribe(PanicOnNext(Some(panic_on_drop)));
    });
    assert_eq!(connections.load(Ordering::SeqCst), 0);

    // The controller is back, so the next subscriber connects. It joins the subject the panic
    // killed, which is the subject's business, so only the connection is checked.
    let (_, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    drop(subscription);
    assert!(is_disconnected.read());
    drop(observable);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_connection_gives_the_controller_back() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let fuse: Arc<Mutex<Option<PanicOnDrop>>> = Arc::new(Mutex::new(None));
    let observable = {
        let connections = connections.clone();
        let is_disconnected = is_disconnected.clone();
        let fuse = fuse.clone();
        Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
                connections.fetch_add(1, Ordering::SeqCst);
                // Unwinds out of the connection while the fuse holds a panic.
                drop(fuse.take_value());
                let _ = observer.on_next(1);
                let is_disconnected = is_disconnected.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    is_disconnected.write(true);
                }))
            },
        )
    }
    .share();

    let observable_of_panic = observable.clone();
    expect_panic_on_drop(|panic_on_drop| {
        drop(fuse.replace_value(Some(panic_on_drop)));
        let (_, observer) = Checker::new();
        let _subscription = observable_of_panic.subscribe(observer);
    });
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    // The controller is back, disconnected, so the next subscriber connects again.
    let (checker, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 2);
    assert_eq!(checker.values(), [1]);

    // The subscriber whose connection panicked is counted out, so this one is the last.
    drop(subscription);
    assert!(is_disconnected.read());
    drop(observable);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_disconnection_gives_the_controller_back() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    let connections = Arc::new(AtomicUsize::new(0));
    let disconnections = Arc::new(AtomicUsize::new(0));
    let fuse: Arc<Mutex<Option<PanicOnDrop>>> = Arc::new(Mutex::new(None));
    let observable = {
        let connections = connections.clone();
        let disconnections = disconnections.clone();
        let fuse = fuse.clone();
        Create::shared_boxed(
            move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
                connections.fetch_add(1, Ordering::SeqCst);
                let _ = observer.on_next(1);
                let disconnections = disconnections.clone();
                let fuse = fuse.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    disconnections.fetch_add(1, Ordering::SeqCst);
                    // Unwinds out of the disconnection while the fuse holds a panic.
                    drop(fuse.take_value());
                }))
            },
        )
    }
    .share();

    let (_, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    expect_panic_on_drop(|panic_on_drop| {
        drop(fuse.replace_value(Some(panic_on_drop)));
        drop(subscription);
    });
    assert_eq!(disconnections.load(Ordering::SeqCst), 1);

    // The controller is back, disconnected, so the next subscriber connects again.
    let (checker, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 2);
    assert_eq!(checker.values(), [1]);

    drop(subscription);
    assert_eq!(disconnections.load(Ordering::SeqCst), 2);
    drop(observable);
}

/// Delegates to `source`, but unwinds out of a clone while `fuse` holds a panic: the clone a
/// connection makes of its source.
#[cfg(panic = "unwind")]
struct PanicOnClone<OE> {
    source: OE,
    fuse: Arc<Mutex<Option<crate::tests_utils::panic::PanicOnDrop>>>,
}

#[cfg(panic = "unwind")]
impl<OE: Clone> Clone for PanicOnClone<OE> {
    fn clone(&self) -> Self {
        drop(self.fuse.take_value());
        Self {
            source: self.source.clone(),
            fuse: self.fuse.clone(),
        }
    }
}

#[cfg(panic = "unwind")]
impl<OE: rx_rust::observable::ObservableTypes> rx_rust::observable::ObservableTypes
    for PanicOnClone<OE>
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

#[cfg(panic = "unwind")]
impl<OE, OR> Observable<OR> for PanicOnClone<OE>
where
    OE: Observable<OR>,
    OR: Observer<OE::Item, OE::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(observer)
    }
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_source_clone_gives_the_controller_back() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let fuse = Arc::new(Mutex::new(None));
    let observable = PanicOnClone {
        source: connection_counting_source!(connections.clone(), is_disconnected.clone()),
        fuse: fuse.clone(),
    }
    .share();

    let observable_of_panic = observable.clone();
    expect_panic_on_drop(|panic_on_drop| {
        drop(fuse.replace_value(Some(panic_on_drop)));
        let (_, observer) = Checker::new();
        let _subscription = observable_of_panic.subscribe(observer);
    });
    // The source was never subscribed: the clone unwound first.
    assert_eq!(connections.load(Ordering::SeqCst), 0);

    // The controller is back, disconnected, so the next subscriber connects.
    let (checker, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    assert_eq!(checker.values(), [1]);

    // The subscriber whose connection panicked is counted out, so this one is the last.
    drop(subscription);
    assert!(is_disconnected.read());
    drop(observable);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_subscriber_does_not_connect_while_unwinding() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let observable = connection_counting_source!(connections.clone(), is_disconnected.clone())
        .publish_behavior(0)
        .ref_count();

    let waiting = Arc::new(Mutex::new(None));
    let observable_of_panic = observable.clone();
    let observable_of_waiting = observable.clone();
    let waiting_of_panic = waiting.clone();
    expect_panic_on_drop(move |panic_on_drop| {
        let mut panic_on_drop = Some(panic_on_drop);
        let mut observable_of_waiting = Some(observable_of_waiting);
        let _subscription = observable_of_panic.subscribe_with_callback(
            move |_| {
                // The subject hands `0` over inside `subscribe`, while this first subscriber holds
                // the controller it was about to connect: a second subscriber joins and waits for
                // that connection, then this one panics.
                if let Some(observable) = observable_of_waiting.take() {
                    let (_, observer) = Checker::new();
                    drop(waiting_of_panic.replace_value(Some(observable.subscribe(observer))));
                }
                drop(panic_on_drop.take());
            },
            |_| {},
        );
    });
    // The second subscriber still waits, but the unwinding thread did not connect for it.
    assert_eq!(connections.load(Ordering::SeqCst), 0);

    // The next subscription connects. It joins the subject the panic killed, which is the
    // subject's business, so only the connection is checked.
    let (_, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    // The waiting subscriber is still counted, so it is the last one that disconnects.
    drop(subscription);
    assert!(!is_disconnected.read());
    drop(waiting.take_value());
    assert!(is_disconnected.read());
    drop(observable);
}

/// Drops what is parked in it when it is dropped itself, whoever else holds the slot.
#[cfg(panic = "unwind")]
struct DropParked<T>(Arc<Mutex<Option<T>>>);

#[cfg(panic = "unwind")]
impl<T> Drop for DropParked<T> {
    fn drop(&mut self) {
        drop(self.0.take_value());
    }
}

#[cfg(panic = "unwind")]
#[test]
fn test_disposal_does_not_connect_while_unwinding() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let connections = Arc::new(AtomicUsize::new(0));
    let is_disconnected = Arc::new(AtomicBool::new(false));
    let observable = connection_counting_source!(connections.clone(), is_disconnected.clone())
        .publish_behavior(0)
        .ref_count();

    let waiting = Arc::new(Mutex::new(None));
    let observable_of_panic = observable.clone();
    let observable_of_waiting = observable.clone();
    let waiting_of_panic = waiting.clone();
    expect_panic_on_drop(move |panic_on_drop| {
        // Declared before the panicking subscription, so the unwind drops it once that
        // subscription has counted itself out and given the controller back.
        let leaving = DropParked(Arc::new(Mutex::new(None)));
        let leaving_of_panic = leaving.0.clone();
        let mut panic_on_drop = Some(panic_on_drop);
        let mut observable_of_waiting = Some(observable_of_waiting);
        let _subscription = observable_of_panic.subscribe_with_callback(
            move |_| {
                // The subject hands `0` over inside `subscribe`, while this first subscriber holds
                // the controller it was about to connect: two more subscribers join and wait for
                // that connection, then this one panics.
                if let Some(observable) = observable_of_waiting.take() {
                    let (_, observer) = Checker::new();
                    let subscription = observable.clone().subscribe(observer);
                    drop(waiting_of_panic.replace_value(Some(subscription)));
                    let (_, observer) = Checker::new();
                    drop(leaving_of_panic.replace_value(Some(observable.subscribe(observer))));
                }
                drop(panic_on_drop.take());
            },
            |_| {},
        );
    });
    // The unwind disposed of one waiting subscriber, which left the other one waiting, but did
    // not connect for it.
    assert_eq!(connections.load(Ordering::SeqCst), 0);

    // The next subscription connects. It joins the subject the panic killed, which is the
    // subject's business, so only the connection is checked.
    let (_, observer) = Checker::new();
    let subscription = observable.clone().subscribe(observer);
    assert_eq!(connections.load(Ordering::SeqCst), 1);

    // The subscriber disposed during the unwind is counted out, and the waiting one is still
    // counted, so it is the last one that disconnects.
    drop(subscription);
    assert!(!is_disconnected.read());
    drop(waiting.take_value());
    assert!(is_disconnected.read());
    drop(observable);
}
