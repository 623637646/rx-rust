mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::range::Range,
    subscription::disposable::Disposable,
};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed_range() {
    let source = 100..103;
    let observable = Range::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100, 101, 102]);
    assert!(checker.is_completed());
}

#[test]
fn test_completed_range_inclusive() {
    let source = 100..=103;
    let observable = Range::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100, 101, 102, 103]);
    assert!(checker.is_completed());
}

#[test]
fn test_async() {
    block_on(async {
        let source = 100..103;
        let observable = Range::new(source);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let handle = spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert_eq!(checker.values(), [100, 101, 102]);
        assert!(checker.is_completed());

        let handle = spawn(async { subscription.dispose() });
        handle.await.unwrap();
        assert_eq!(checker.values(), [100, 101, 102]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let source = 100..103;
    let observable = Range::new(source);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [100, 101, 102]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [100, 101, 102]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let source = 100..103;
    let observable = Range::new(source);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = 100..103;
    let observable = Range::new(source);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = 100..103;
    let observable = Range::new(source);

    observable.filter(|_| true);
}
