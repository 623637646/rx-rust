mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::interval::Interval,
    scheduler::tokio_scheduler::TokioScheduler,
};
use std::time::Duration;
use tests_utils::checker::Checker;

#[tokio::test]
async fn test_completed_no_delay() {
    let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[0]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_active());

    subscription.unsubscribe();

    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_completed_with_delay() {
    let observable = Interval::new(
        Duration::from_millis(100),
        TokioScheduler,
        Some(Duration::from_millis(100)),
    );
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_active());

    subscription.unsubscribe();

    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_unsubscribe() {
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Interval::new(
        Duration::from_millis(100),
        TokioScheduler,
        Some(Duration::from_millis(100)),
    );
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();

    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[0, 1, 2, 3]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[0, 1, 2, 3, 4]));
    assert!(checker_2.is_active());

    _ = subscription_2; // keep the subscription alive
}

#[tokio::test]
async fn test_async() {
    let observable = Interval::new(
        Duration::from_millis(100),
        TokioScheduler,
        Some(Duration::from_millis(100)),
    );
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();

    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker.is_values_matched(&[0, 1, 2]));
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_subscribe_by_different_observer() {
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Interval::new(
        Duration::from_millis(100),
        TokioScheduler,
        Some(Duration::from_millis(100)),
    );
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    subscription_2.unsubscribe();

    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(checker_1.is_values_matched(&[0, 1, 2]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[0, 1, 2]));
    assert!(checker_2.is_dropped());
}

#[tokio::test]
async fn test_clone() {
    let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);
    let _ = observable.clone();
}

#[tokio::test]
async fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[tokio::test]
async fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Interval::new(Duration::from_millis(100), TokioScheduler, None);

    let _ = observable.buffer_with_count(1);
}
