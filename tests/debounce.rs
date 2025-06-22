mod tests_utils;

use crate::tests_utils::test_channel::test_channel;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, never::Never},
        filtering::debounce::Debounce,
    },
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_scheduler::TestScheduler, test_struct::TestStruct};

#[tokio::test]
async fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());

    tokio::time::sleep(Duration::from_millis(110)).await;
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[tokio::test]
async fn test_completed_then_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_next(222);
    subject.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_active());

    subject.on_next(444);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());

    tokio::time::sleep(Duration::from_millis(110)).await;
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(444);
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));

    tokio::time::sleep(Duration::from_millis(110)).await;
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[tokio::test]
async fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();
    let observable_3 = observable_2.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    subject.on_next(111);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    subscription_2.unsubscribe();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111, 222, 333]);
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111, 222, 333]);
    assert!(checker_3.is_error("error"));
}

#[tokio::test]
async fn test_async() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async move {
        sender.on_next(&111);
        sender
    });
    let _sender = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [&111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [&111]);
    assert!(checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[tokio::test]
async fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
}

#[tokio::test]
async fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .debounce(Duration::from_millis(50), TestScheduler)
        .debounce(Duration::from_millis(150), TestScheduler);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(190)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(0);
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(0);
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(0);
    tokio::time::sleep(Duration::from_millis(125)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    tokio::time::sleep(Duration::from_millis(175)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(333);
    tokio::time::sleep(Duration::from_millis(225)).await;
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[tokio::test]
async fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Debounce::new(observable, Duration::from_millis(100), TestScheduler);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    sender.on_next(333);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(90)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(checker.values(), [111, 333]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(444);
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());

    tokio::time::sleep(Duration::from_millis(110)).await;
    assert_eq!(checker.values(), [111, 333, 444]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[tokio::test]
async fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.debounce(Duration::from_millis(10), TestScheduler);

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;
}

#[tokio::test]
async fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.debounce(Duration::from_millis(100), TestScheduler);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[tokio::test]
async fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Never.debounce(Duration::from_millis(100), TestScheduler);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[tokio::test]
async fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Never.debounce(Duration::from_millis(100), TestScheduler);

    observable.filter(|_| true);
}
