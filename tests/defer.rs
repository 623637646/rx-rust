mod tests_utils;

use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::{Mutable, MutableHelper, Shared};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::creating::{create::Create, defer::Defer, just::Just},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| observable);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_active());

    subject.clone().on_termination(Termination::Error(&error));
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

    // Custom operations
    let observable = Defer::new(|| observable);

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

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| observable);
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
        let observable = Defer::new(|| observable);

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
        let observable = Defer::new(|| observable);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = Defer::new(|| {
        s.consume();
        observable
    });

    observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Defer::new(|| Just::new(TestStruct));
    _ = observable.clone(); // Make sure it's Clone when OE is not Clone.
}

#[test]
fn test_boxed_observable() {
    // Custom operations
    let switch = Shared::new(Mutable::new(false));
    let observable = Defer::new(|| {
        let observable = Just::new(111);
        if *switch.lock_ref() {
            observable.into_boxed()
        } else {
            observable.map(|value| value * 2).into_boxed()
        }
    });
    let (checker, observer) = Checker::new();
    let _subscription = observable.clone().subscribe(observer);
    assert_eq!(checker.values(), [222]);
    assert!(checker.is_completed());

    *switch.lock_mut() = true;
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = Defer::new(|| subject);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = Defer::new(|| subject);

    observable.filter(|_| true);
}
