mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::empty::Empty,
};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let observable = Empty;
    let (checker, observer) = Checker::<i32, Infallible>::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_async() {
    let observable = Empty;
    let (checker, observer) = Checker::<i32, Infallible>::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Empty;
    let (checker_1, observer_1) = Checker::<i32, Infallible>::new();
    let (checker_2, observer_2) = Checker::<i32, Infallible>::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let observable = Empty;
    let _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Empty;

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::<Vec<i32>, Infallible>::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Empty;

    let _ = observable.buffer_with_count(1);
}
