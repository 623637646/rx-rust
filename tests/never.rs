mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::never::Never,
    subscription::disposable::Disposable,
};
use tests_utils::checker::Checker;

#[test]
fn test_async() {
    block_on(async {
        let observable = Never;
        let (checker, observer) = Checker::new();

        let handle = spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());

        let handle = spawn(async { subscription.dispose() });
        handle.await.unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Never;
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_dropped());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_dropped());
}

#[test]
fn test_clone() {
    let observable = Never;
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Never;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Never;

    observable.filter(|_| true);
}
