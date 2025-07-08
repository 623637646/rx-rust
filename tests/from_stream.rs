mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, sleep, spawn};
use futures::{SinkExt, stream};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::from_stream::FromStream,
    subscription::disposable::Disposable,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};

#[test]
fn test_completed() {
    block_on(async {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        tx.send(111).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        drop(tx);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_without_next() {
    block_on(async {
        let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        drop(tx);
        sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_completed());
    });
}

#[test]
fn test_unsubscribe() {
    block_on(async {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        tx.send(111).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        subscription.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_async() {
    block_on(async {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        tx.send(111).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        drop(tx);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(async {
        let source = stream::iter(vec![111, 222, 333]);

        let observable = FromStream::new(source, TestScheduler);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker_1.values(), [111, 222, 333]);
        assert!(checker_1.is_completed());
        assert_eq!(checker_2.values(), [111, 222, 333]);
        assert!(checker_2.is_completed());
    });
}

#[test]
fn test_complete_after_next() {
    block_on(async {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        sleep(Duration::from_millis(10)).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        tx.send(111).await.unwrap();
        drop(tx);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());

        sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());

        sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(async {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        sleep(Duration::from_millis(10)).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        tx.send(111).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());

        sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());

        sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(async {
        let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        sleep(Duration::from_millis(10)).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        drop(tx);
        sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_completed());

        sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_completed());

        sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(async {
        let (_tx, rx) = futures::channel::mpsc::unbounded::<i32>();
        let stream = rx;
        let observable = FromStream::new(stream, TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
    });
}

#[test]
fn test_clone() {
    let source = stream::iter(vec![111, 222, 333]);
    let observable = FromStream::new(source, TestScheduler);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(async {
        // Custom operations
        let source = stream::iter(vec![111, 222, 333]);
        let observable = FromStream::new(source, TestScheduler);

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(async {
        // Custom operations
        let source = stream::iter(vec![111, 222, 333]);
        let observable = FromStream::new(source, TestScheduler);

        observable.filter(|_| true);
    });
}
