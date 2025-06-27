mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::interval::Interval,
    subscription::disposable::Disposable,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};

#[tokio::test]
async fn test_completed_no_delay() {
    let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(checker.values(), [0]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_active());

    subscription.dispose();
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_completed_with_delay() {
    let observable = Interval::new(
        Duration::from_millis(100),
        TestScheduler,
        Some(Duration::from_millis(100)),
    );
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_active());

    subscription.dispose();
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_unsubscribe() {
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Interval::new(
        Duration::from_millis(100),
        TestScheduler,
        Some(Duration::from_millis(100)),
    );
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_active());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 1, 2, 3]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 1, 2, 3, 4]);
    assert!(checker_2.is_active());
}

#[tokio::test]
async fn test_async() {
    let observable = Interval::new(
        Duration::from_millis(100),
        TestScheduler,
        Some(Duration::from_millis(100)),
    );
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1]);
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.dispose() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker.values(), [0, 1, 2]);
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_subscribe_by_different_observer() {
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = Interval::new(
        Duration::from_millis(100),
        TestScheduler,
        Some(Duration::from_millis(100)),
    );
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_active());

    subscription_1.dispose();
    subscription_2.dispose();
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_dropped());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert!(checker_2.is_dropped());
}

#[tokio::test]
async fn test_undisposed_schedule() {
    let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
}

#[tokio::test]
async fn test_clone() {
    let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);
    _ = observable.clone();
}

#[tokio::test]
async fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[tokio::test]
async fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);

    observable.filter(|_| true);
}
