mod tests_utils;

use futures::stream;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::from_stream::FromStream,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[tokio::test]
async fn test_completed() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = UnboundedReceiverStream::new(rx);
    let observable = FromStream::new(stream, TestScheduler);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tx.send(111).unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    drop(tx);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_unsubscribe() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = UnboundedReceiverStream::new(rx);
    let observable = FromStream::new(stream, TestScheduler);
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tx.send(111).unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subscription.unsubscribe();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_dropped());
}

#[tokio::test]
async fn test_async() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = UnboundedReceiverStream::new(rx);
    let observable = FromStream::new(stream, TestScheduler);
    let (checker, observer) = Checker::new();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let _subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tx.send(111).unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    drop(tx);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
}

#[tokio::test]
async fn test_subscribe_by_different_observer() {
    let source = stream::iter(vec![111, 222, 333]);

    let observable = FromStream::new(source, TestScheduler);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(checker_1.is_values_matched(&[111, 222, 333]));
    assert!(checker_1.is_completed());
    assert!(checker_2.is_values_matched(&[111, 222, 333]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_clone() {
    let source = stream::iter(vec![111, 222, 333]);
    let observable = FromStream::new(source, TestScheduler);
    _ = observable.clone();
}

#[tokio::test]
async fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = stream::iter(vec![111, 222, 333]);
    let observable = FromStream::new(source, TestScheduler);

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[tokio::test]
async fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = stream::iter(vec![111, 222, 333]);
    let observable = FromStream::new(source, TestScheduler);

    observable.buffer_with_count(1);
}
