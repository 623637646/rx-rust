mod tests_utils;

use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::repeat::Repeat,
};
use std::{convert::Infallible, time::Duration};
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let observable = Repeat::new(3, 4);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [3, 3, 3, 3]);
    assert!(checker.is_completed());
}

#[test]
fn test_completed_n_0() {
    let observable = Repeat::new(3, 0);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_completed());
}

#[test]
fn test_ref() {
    let value = 1;
    let observable = Repeat::new(&value, 4);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value, &value, &value, &value]);
    assert!(checker.is_completed());
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let observable = Repeat::new(3, 4);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [3, 3, 3, 3]);
        assert!(checker.is_completed());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [3, 3, 3, 3]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Repeat::new(3, 4);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [3, 3, 3, 3]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [3, 3, 3, 3]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let observable = Repeat::new(3, 4);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Repeat::new(3, 4);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Repeat::new(3, 4);

    observable.filter(|_| true);
}
