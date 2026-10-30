mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::DropProbe;
use crate::tests_utils::test_channel::{ChannelState, test_channels};
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::timer::Timer;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::utility::hold::Hold,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel};

/// What happened, in order: the events the observer saw, and the drop of the resource.
type Events = Arc<Mutex<Vec<&'static str>>>;

/// A resource that records its drop in `events`.
fn resource(events: &Events) -> DropProbe {
    labelled(events, "dropped")
}

/// A resource that records its drop in `events` as `label`.
fn labelled(events: &Events, label: &'static str) -> DropProbe {
    let events = events.clone();
    DropProbe::new().on_drop(Box::new(move || {
        events.with_mut(|events| events.push(label));
    }))
}

#[test]
fn test_completed() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.hold(resource(&events));
    let _subscription = observable.subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(events.clone_value().is_empty());

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert!(events.clone_value().is_empty());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_error() {
    let events = Events::default();
    let (mut sender, observable, _channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.hold(resource(&events)).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("boom".to_owned()));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("boom".to_owned()));
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable.hold(resource(&events)).subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value).is_continue());
    assert_eq!(checker.values(), [&value]);
    assert!(events.clone_value().is_empty());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;
    let events = Events::default();

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });

    let _subscription = observable.hold(resource(&events)).subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                *error *= 2;
            }
        },
    );

    assert_eq!(value, 222);
    assert_eq!(error, 444);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_subscribe_by_different_observer() {
    // Each subscription holds a clone of the resource, and drops it when that subscription ends.
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let resource = Arc::new(());

    let observable_1 = observable.hold(resource.clone());
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(Arc::strong_count(&resource), 3);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [222]);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(Arc::strong_count(&resource), 2);

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(Arc::strong_count(&resource), 1);
}

#[test]
fn test_multiple_operation() {
    // Two resources on one chain: the one held nearer the observer is dropped first, as the
    // termination comes back out of it.
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .hold(labelled(&events, "upstream dropped"))
        .hold(labelled(&events, "downstream dropped"))
        .subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert!(events.clone_value().is_empty());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        events.clone_value(),
        ["downstream dropped", "upstream dropped"]
    );
}

#[test]
fn test_dropped_after_termination() {
    // The termination is delivered before the resource is dropped.
    let events = Events::default();
    let (sender, observable, _channel_checker) = test_channel::<i32, Infallible>();

    let _subscription = observable
        .hold(resource(&events))
        .subscribe_with_callback(|_| {}, {
            let events = events.clone();
            move |_| events.with_mut(|events| events.push("terminated"))
        });
    sender.on_termination(Termination::Completed);
    assert_eq!(events.clone_value(), ["terminated", "dropped"]);
}

#[test]
fn test_unsubscribe() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.hold(resource(&events)).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(events.clone_value().is_empty());

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_abandon_after_next() {
    // A source that lets go of its observer without a termination drops the resource with it.
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.hold(resource(&events)).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_completed_on_subscribe() {
    // A source that ends while it is subscribed to drops the resource before `subscribe` returns.
    let events = Events::default();
    let (checker, observer) = Checker::new();

    let _subscription = Just::new(111).hold(resource(&events)).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_never_subscribed() {
    let events = Events::default();
    let observable = Just::new(111).hold(resource(&events));
    assert!(events.clone_value().is_empty());

    drop(observable);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_unsub_on_next_by_take() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .hold(resource(&events))
        .take(1)
        .subscribe(observer);
    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let events = Events::default();
        let (checker, observer) = Checker::new();
        let observable = Timer::new(111, DURATION_10_MS, scheduler.clone()).hold(resource(&events));

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(events.clone_value().is_empty());

        scheduler.sleep(DURATION_10_MS * 3).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(events.clone_value(), ["dropped"]);
    });
}

#[test]
fn test_without_convenient_api() {
    let events = Events::default();
    let (checker, observer) = Checker::new();
    let _subscription = Hold::new(Just::new(111), resource(&events)).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_clone() {
    let observable = Just::new(111).hold(Arc::new(()));
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(111).hold(());
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    Just::new(111).hold(());
}
