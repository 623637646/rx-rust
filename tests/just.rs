mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::just::Just,
};
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let observable = Just::new(111);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
}

#[test]
fn test_ref() {
    let value = 111;

    let observable = Just::new(&value);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&value]));
    assert!(checker.is_completed());
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let observable = Just::new(&mut value);
    let (checker, observer) = Checker::new();

    let (mut on_next, on_terminal) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_terminal,
    );

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
    assert_eq!(value, 222);
}

#[tokio::test]
async fn test_async() {
    let observable = Just::new(111);
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Just::new(111);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let observable = Just::new(111);
    let _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Just::new(111);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Just::new(111);

    let _ = observable.buffer_with_count(1);
}
