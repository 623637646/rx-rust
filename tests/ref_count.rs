mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt, ref_count_observable::RefCount},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_2.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2, 3]);
    assert!(checker_2.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
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
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [&value_2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [&value_2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [&value_2]);
    assert!(checker_2.is_error(&error));
    assert!(channel_checker.is_error(&error));
}

#[tokio::test]
async fn test_async() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(move |_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let handle = tokio::spawn(async move { observable_1.subscribe(observer_1) });
    let subscription_1 = handle.await.unwrap();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move {
        sender.on_next(());
        sender
    });
    let mut sender = handle.await.unwrap();
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move { observable_2.subscribe(observer_2) });
    let _subscription_2 = handle.await.unwrap();
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move {
        sender.on_next(());
        sender
    });
    let sender = handle.await.unwrap();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move { subscription_1.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move { sender.on_termination(Termination::Error("error")) });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count()
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_without_convenient_api() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish();
    let observable = RefCount::new(observable);
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_complete_on_next() {
    let mut counter = 0;
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = Some(subject.clone());
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .take()
                .unwrap()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(());
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_completed());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_completed());
}

#[test]
fn test_error_on_next() {
    let mut counter = 0;
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = Some(subject.clone());
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .take()
                .unwrap()
                .on_termination(Termination::Error("error"));
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(());
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_error("error"));

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsub_on_next() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_initialized());

    // no unsubscribe
    let subscription = observable_1.subscribe(observer_1);

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_next(move |value, callback| {
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
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
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_dropped());
    assert!(channel_checker.is_subscribed());

    subscription.unsubscribe();
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_unsub_on_completed() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_initialized());

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_termination(move |value, callback| {
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_3
            .hook_on_termination(move |value, callback| {
                callback(value);
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_completed());
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_unsub_on_error() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .publish()
        .ref_count();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();
    let observable_3 = observable;

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_initialized());

    // no unsubscribe
    let _subscription = observable_1.subscribe(observer_1);

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_2
            .hook_on_termination(move |value, callback| {
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable_3
            .hook_on_termination(move |value, callback| {
                callback(value);
                sub_cloned.lock().unwrap().take().unwrap().unsubscribe();
            })
            .subscribe(observer_3),
    );

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_error("error"));
    assert_eq!(checker_3.values(), [1]);
    assert!(checker_3.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_share_api() {
    let mut counter = 0;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable
        .map(|_| {
            counter += 1;
            counter
        })
        .share();
    let observable_1 = observable.clone();
    let observable_2 = observable.clone();

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_initialized());

    let subscription_1 = observable_1.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(());
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_completed());
    assert!(channel_checker.is_completed());
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
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
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
    let mut life_marker = None;

    // Error
    // let mut life_marker = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker = Some(observer);
            Subscription::new_none_disposal()
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

        let observable = Create::new(|_: BoxedObserver<'_, &TestStruct, Infallible>| {
            Subscription::new_none_disposal()
        });
        let observable = observable.publish().ref_count();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.publish().ref_count();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.publish().ref_count();

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.publish().ref_count();

    observable.buffer_with_count(1);
}
