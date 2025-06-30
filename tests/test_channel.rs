mod tests_utils;

use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::subscription::Subscription;
use rx_rust::subscription::disposable::Disposable;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::checker::Checker;
use tests_utils::test_channel::test_channel;

#[test]
fn test_unsub_on_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, _, Infallible>();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_next(move |value, callback| {
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_3
            .hook_on_next(move |value, callback| {
                callback(value);
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());
    assert!(channel_checker_3.is_subscribed());

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_dropped());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_unsubscribed());
    assert!(channel_checker_3.is_unsubscribed());
}

#[test]
fn test_unsub_on_completed() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, _, Infallible>();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_termination(move |value, callback| {
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_3
            .hook_on_termination(move |value, callback| {
                callback(value);
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());
    assert!(channel_checker_3.is_subscribed());

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());
    assert!(channel_checker_3.is_subscribed());

    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Completed);
    sender_3.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_completed());
    assert!(channel_checker_1.is_completed());
    assert!(channel_checker_2.is_completed());
    assert!(channel_checker_3.is_completed());
}

#[test]
fn test_unsub_on_error() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_termination(move |value, callback| {
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_3
            .hook_on_termination(move |value, callback| {
                callback(value);
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());
    assert!(channel_checker_3.is_subscribed());

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());
    assert!(channel_checker_1.is_subscribed());
    assert!(channel_checker_2.is_subscribed());
    assert!(channel_checker_3.is_subscribed());

    sender_1.on_termination(Termination::Error("error"));
    sender_2.on_termination(Termination::Error("error"));
    sender_3.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_error("error"));
    assert!(channel_checker_1.is_error("error"));
    assert!(channel_checker_2.is_error("error"));
    assert!(channel_checker_3.is_error("error"));
}
