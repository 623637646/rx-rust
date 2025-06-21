mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::from_future::FromFuture,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};

#[tokio::test]
async fn test_completed() {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let observable = FromFuture::new(rx, TestScheduler);
    let observable = observable.map(|result| result.unwrap_or(-1));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tx.send(111).unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_completed_drop() {
    let (tx, rx) = tokio::sync::oneshot::channel::<i32>();

    let observable = FromFuture::new(rx, TestScheduler);
    let observable = observable.map(|result| result.unwrap_or(-1));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    drop(tx);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(checker.values(), [-1]);
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_unsubscribe() {
    let (_tx, rx) = tokio::sync::oneshot::channel::<i32>();

    let observable = FromFuture::new(rx, TestScheduler);
    let observable = observable.map(|result| result.unwrap_or(-1));
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subscription.unsubscribe();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_async() {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let observable = FromFuture::new(rx, TestScheduler);
    let observable = observable.map(|result| result.unwrap_or(-1));
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let _subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let handle = tokio::spawn(async move { tx.send(111).unwrap() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_subscribe_by_different_observer() {
    let source = std::future::ready(111);

    let observable = FromFuture::new(source, TestScheduler);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let source = std::future::ready(111);
    let observable = FromFuture::new(source, TestScheduler);
    _ = observable.clone();
}

#[tokio::test]
async fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = async { 111 };
    let observable = FromFuture::new(source, TestScheduler);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[tokio::test]
async fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = async { 111 };
    let observable = FromFuture::new(source, TestScheduler);

    observable.filter(|_| true);
}
