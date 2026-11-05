mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::DropProbe;
use crate::tests_utils::test_channel::{ChannelState, test_channels};
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::timer::Timer;
use rx_rust::operators::error_handling::retry::RetryAction;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::utility::using::Using,
};
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel};

/// What happened, in order: the creation and the drop of each resource, and the events the
/// observer saw.
type Events = Arc<Mutex<Vec<&'static str>>>;

/// A resource that records its drop in `events`.
fn resource(events: &Events) -> DropProbe {
    let events = events.clone();
    DropProbe::new().on_drop(Box::new(move || {
        events.with_mut(|events| events.push("dropped"));
    }))
}

/// A resource factory that records each resource it creates in `events`, and its drop.
fn factory<E>(events: &Events) -> impl FnOnce() -> Result<DropProbe, E> + Clone + Send + use<E> {
    let events = events.clone();
    move || {
        events.with_mut(|events| events.push("created"));
        Ok(resource(&events))
    }
}

#[test]
fn test_completed() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = Using::new(factory(&events), |_| observable);
    assert!(events.clone_value().is_empty());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(events.clone_value(), ["created"]);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(events.clone_value(), ["created"]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_error() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(factory(&events), |_| observable).subscribe(observer);
    assert_eq!(events.clone_value(), ["created"]);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_unsubscribe() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = Using::new(factory(&events), |_| observable).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(events.clone_value(), ["created"]);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(factory(&events), |_| observable).subscribe(observer);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value).is_continue());
    assert_eq!(checker.values(), [&value]);
    assert_eq!(events.clone_value(), ["created"]);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(events.clone_value(), ["created", "dropped"]);
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

    let _subscription = Using::new(factory(&events), |_| observable).subscribe_with_callback(
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
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_subscribe_by_different_observer() {
    // Each subscription creates a resource of its own, and drops it when that subscription ends.
    let events = Events::default();
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = Using::new(factory(&events), move |_| observable);
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    assert_eq!(events.clone_value(), ["created"]);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(events.clone_value(), ["created", "created"]);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [222]);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(events.clone_value(), ["created", "created", "dropped"]);

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        events.clone_value(),
        ["created", "created", "dropped", "dropped"]
    );
}

#[test]
fn test_stop_on_next() {
    // An observer that ends its own stream is dropped by the observable, and the resource with it.
    let events = Events::default();
    let (mut sender, observable, _channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = Using::new(factory(&events), |_| observable).subscribe(observer);
    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_unsub_on_next_by_take() {
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(factory(&events), |_| observable)
        .take(1)
        .subscribe(observer);
    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_abandon_after_next() {
    // An observable that lets go of its observer without a termination drops the resource with it.
    let events = Events::default();
    let (mut sender, observable, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(factory(&events), |_| observable).subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.abandon();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_completed_on_subscribe() {
    // An observable that ends while it is subscribed to drops the resource before `subscribe`
    // returns.
    let events = Events::default();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(factory(&events), |_| Just::new(111)).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(events.clone_value(), ["created", "dropped"]);
}

#[test]
fn test_never_subscribed() {
    // No subscription, no resource.
    let events = Events::default();
    let observable = Using::new(factory(&events), |_| Just::new(111));

    drop(observable);
    assert!(events.clone_value().is_empty());
}

#[test]
fn test_resource_error() {
    // A resource that cannot be created ends the subscription with its error, and the observable
    // is never built.
    let built = Arc::new(AtomicBool::new(false));
    let (_sender, observable, channel_checker) = test_channel::<i32, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(|| Err::<(), _>("error"), {
        let built = built.clone();
        move |_| {
            built.store(true, Ordering::SeqCst);
            observable
        }
    })
    .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
    assert!(!built.load(Ordering::SeqCst));
}

#[test]
fn test_observable_from_resource() {
    // The observable factory gets the resource by `&mut`, and takes what it needs out of it.
    let events = Events::default();
    let (checker, observer) = Checker::new();

    let _subscription = Using::new(
        {
            let events = events.clone();
            move || Ok((resource(&events), vec![111, 222]))
        },
        |(_, values)| FromIter::new(std::mem::take(values)),
    )
    .subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(events.clone_value(), ["dropped"]);
}

#[test]
fn test_dropped_before_termination() {
    // The resource is dropped before the termination is delivered.
    let events = Events::default();
    let (sender, observable, _channel_checker) = test_channel::<i32, Infallible>();

    let _subscription =
        Using::new(factory(&events), |_| observable).subscribe_with_callback(|_| {}, {
            let events = events.clone();
            move |_| events.with_mut(|events| events.push("terminated"))
        });
    sender.on_termination(Termination::Completed);
    assert_eq!(events.clone_value(), ["created", "dropped", "terminated"]);
}

#[test]
fn test_dropped_before_retry() {
    // A retry subscribes again from the termination, so the next resource is created only once the
    // previous one is dropped.
    let events = Events::default();
    let (channels, observable) = test_channels::<i32, &str>();
    let (checker, observer) = Checker::new();

    let source = Using::new(factory(&events), move |_| observable);
    let _subscription = source
        .clone()
        .retry(move |_| RetryAction::Retry(source.clone()))
        .subscribe(observer);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(events.clone_value(), ["created"]);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(events.clone_value(), ["created", "dropped", "created"]);

    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        events.clone_value(),
        ["created", "dropped", "created", "dropped"]
    );
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let events = Events::default();
        let (checker, observer) = Checker::new();
        let observable = Using::new(factory(&events), {
            let scheduler = scheduler.clone();
            move |_| Timer::new(111, DURATION_10_MS, scheduler)
        });

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(events.clone_value(), ["created"]);

        scheduler.sleep(DURATION_10_MS * 3).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(events.clone_value(), ["created", "dropped"]);
    });
}

#[test]
fn test_clone() {
    let observable = Using::new(|| Ok(Arc::new(())), |_| Just::new(111));
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Using::new(|| Ok(()), |_| Just::new(111));
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    Using::new(|| Ok(()), |_| Just::new(111));
}
