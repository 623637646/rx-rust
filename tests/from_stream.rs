mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use futures::{SinkExt, StreamExt, stream};
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::Shared;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    operators::creating::from_stream::FromStream,
};
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
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
fn test_unsubscribe_with_always_ready_stream() {
    block_on(|runtime| async move {
        // `repeat` is infinite and always ready: without a yield between
        // items the task would never hit a pending await point, so disposal
        // could never take effect.
        let stream = stream::repeat(111);
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await;
        assert!(!checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        // Let the abort take effect before sampling the count.
        runtime.sleep(DURATION_10_MS).await;
        let count = checker.values().len();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values().len(), count);
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
fn test_stop_on_next() {
    block_on(|runtime| async move {
        let pulled = Shared::new(AtomicUsize::new(0));
        let pulled_stream = pulled.clone();
        // Infinite and always ready: only the observer's answer can end it.
        let stream = stream::iter(1..).inspect(move |_| {
            pulled_stream.fetch_add(1, Ordering::SeqCst);
        });
        let observable = FromStream::new(stream, runtime.clone());
        let (checker, observer) = Checker::<_, Infallible>::stopping_after(2);

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await;
        // The observer ended its own stream on the second value, so the scheduler stops polling
        // the stream right there and the observer is dropped instead of being completed.
        assert_eq!(checker.values(), [1, 2]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(pulled.load(Ordering::SeqCst), 2);

        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(pulled.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn test_stop_on_next_by_take() {
    block_on(|runtime| async move {
        let pulled = Shared::new(AtomicUsize::new(0));
        let pulled_stream = pulled.clone();
        let stream = stream::iter(1..).inspect(move |_| {
            pulled_stream.fetch_add(1, Ordering::SeqCst);
        });
        let observable = FromStream::new(stream, runtime.clone()).take(2);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await;
        // `take` stops the source once it has its values, so the values behind them are never
        // pulled: only the completion of the operator itself reaches the observer.
        assert_eq!(checker.values(), [1, 2]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(pulled.load(Ordering::SeqCst), 2);
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
        let _ = observable.subscribe(observer);
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
