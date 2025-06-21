mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::creating::create::Create,
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::<String>::Completed);
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_completed_from_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Create::new(|observer| observable.subscribe(observer));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::Error("error"));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_from_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Create::new(|observer| observable.subscribe(observer));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[tokio::test]
async fn test_unsubscribe() {
    let observable = Create::new(|mut observer| {
        observer.on_next(1);
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.on_next(2);
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.on_next(3);
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.on_termination(Termination::<String>::Completed);
        });
        Subscription::new_with_disposal_callback(move || handle.abort())
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [1, 2]);
    assert!(checker_2.is_active());

    subscription_1.unsubscribe(); // unsubscribe

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2, 3]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [1, 2, 3]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_unsubscribe_wrap_observable() {
    let mut subject = PublishSubject::default();
    let subject_cloned = subject.clone();
    let observable = Create::new(|observer| subject_cloned.subscribe(observer));
    let (checker, observer) = Checker::new();

    let subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subscription.unsubscribe();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_dropped());

    subject.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_dropped());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_dropped());
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let observable = Create::new(|mut observer| {
        observer.on_next(&value);
        observer.on_termination(Termination::Error(&error));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value);
        observer.on_termination(Termination::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(*error));
                *error *= 2;
            }
        },
    );

    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[tokio::test]
async fn test_async() {
    let observable = Create::new(|mut observer| {
        observer.on_next(1);
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.on_next(2);
            tokio::time::sleep(Duration::from_millis(100)).await;
            observer.on_termination(Termination::<String>::Completed);
        });
        Subscription::new_with_disposal_callback(move || handle.abort())
    });
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(checker.values(), [1]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [1, 2]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [1, 2]);
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [1, 2]);
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::Error("error"));
        Subscription::new_none_disposal()
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
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
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    Create::new(|mut observer| {
        s.consume();
        observer.on_next(111);
        observer.on_termination(Termination::Error("error"));
        Subscription::new_none_disposal()
    });
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::Error("error"));
        Subscription::new_none_disposal()
    });

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(111);
        observer.on_termination(Termination::Error("error"));
        Subscription::new_none_disposal()
    });

    observable.filter(|_| true);
}
