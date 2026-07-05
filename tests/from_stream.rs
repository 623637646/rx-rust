mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use futures::{SinkExt, stream};
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::from_stream::FromStream,
};
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let source = stream::iter(vec![111, 222, 333]);

        let observable = FromStream::new(source, runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [111, 222, 333]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111, 222, 333]);
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone()).take(1);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let stream = rx;
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            tx.unbounded_send(*i).unwrap();
        }
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let source = stream::iter(vec![111, 222, 333]);
        let observable = FromStream::new(source, runtime.clone());
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let source = stream::iter(vec![111, 222, 333]);
        let observable = FromStream::new(source, runtime.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let source = stream::iter(vec![111, 222, 333]);
        let observable = FromStream::new(source, runtime.clone());

        observable.filter(|_| true);
    });
}
