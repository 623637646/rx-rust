mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Termination,
    operators::creating::from_iter::FromIter,
};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed_array() {
    let source = [1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[1, 2, 3]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_array_ref() {
    let source = [1, 2, 3];

    let observable = FromIter::new(&source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&1, &2, &3]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_array_mut() {
    let mut source = [1, 2, 3];

    let observable = FromIter::new(&mut source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(source, [2, 4, 6]);
}

#[test]
fn test_completed_slice() {
    let source: &[i32] = &[1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&1, &2, &3]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_slice_mut() {
    let mut data = [1, 2, 3];
    let source: &mut [i32] = &mut data;

    let observable = FromIter::new(source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(data, [2, 4, 6]);
}

#[test]
fn test_completed_vec() {
    let source = vec![1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[1, 2, 3]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_vec_ref() {
    let source = vec![1, 2, 3];

    let observable = FromIter::new(&source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&1, &2, &3]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_vec_mut() {
    let mut source = vec![1, 2, 3];

    let observable = FromIter::new(&mut source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(source, [2, 4, 6]);
}

#[test]
fn test_completed_range() {
    let source = 100..103;

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[100, 101, 102]));
    assert!(checker.is_completed());
}

#[test]
fn test_ref() {
    let v1 = 1;
    let v2 = 2;
    let v3 = 3;
    let source = [&v1, &v2, &v3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&v1, &v2, &v3]));
    assert!(checker.is_completed());
}

#[test]
fn test_mut_ref() {
    let mut v1 = 1;
    let mut v2 = 2;
    let mut v3 = 3;
    let source = [&mut v1, &mut v2, &mut v3];

    let observable = FromIter::new(source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(v1, 2);
    assert_eq!(v2, 4);
    assert_eq!(v3, 6);
}

#[tokio::test]
async fn test_async() {
    let source = vec![1, 2, 3];
    let observable = FromIter::new(source);
    let (checker, observer) = Checker::<i32, Infallible>::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[1, 2, 3]));
    assert!(checker.is_completed());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[1, 2, 3]));
    assert!(checker.is_completed());
}

#[test]
fn test_subscribe_by_different_observer() {
    let source = [1, 2, 3];

    let observable = FromIter::new(source);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.is_values_matched(&[1, 2, 3]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[1, 2, 3]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let source = [1, 2, 3];
    let observable = FromIter::new(source);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = [1, 2, 3];
    let observable = FromIter::new(source);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = [1, 2, 3];
    let observable = FromIter::new(source);

    observable.buffer_with_count(1);
}
