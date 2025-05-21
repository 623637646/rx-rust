mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Termination,
    operators::creating::throw::Throw,
};
use tests_utils::checker::Checker;

#[test]
fn test_error() {
    let observable = Throw::new(111);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_error(111));
}

#[test]
fn test_ref() {
    let error = 111;

    let observable = Throw::new(&error);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut error = 111;

    let observable = Throw::new(&mut error);
    let (checker, observer) = Checker::<i32, i32>::new();

    let (_, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |_| unreachable!(),
        |termination| match termination {
            Termination::Completed => unreachable!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(*error));
                *error = 222;
            }
        },
    );

    assert_eq!(checker.values(), []);
    assert!(checker.is_error(111));
    assert_eq!(error, 222);
}

#[tokio::test]
async fn test_async() {
    let observable = Throw::new(111);
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert_eq!(checker.values(), []);
    assert!(checker.is_error(111));

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), []);
    assert!(checker.is_error(111));
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Throw::new(111);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_error(111));
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_error(111));
}

#[test]
fn test_clone() {
    let observable = Throw::new(111);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Throw::new(111);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Throw::new(111);

    observable.buffer_with_count(1);
}
