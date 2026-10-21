mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_scheduler::block_on;
use futures::{SinkExt, StreamExt, stream};
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    operators::creating::from_try_stream::FromTryStream,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_without_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let pulled = Arc::new(AtomicUsize::new(0));
    let pulled_stream = pulled.clone();
    let stream = stream::iter([Ok(111), Err("error"), Ok(222)]).inspect(move |_| {
        pulled_stream.fetch_add(1, Ordering::SeqCst);
    });
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    // The error terminates the observable, so the value behind it is never pulled.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(pulled.load(Ordering::SeqCst), 2);

    time.advance_by(Duration::ZERO);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);
}

#[test]
fn test_error_without_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Err("error")).unwrap();
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsubscribe_with_always_ready_stream() {
    block_on(|scheduler| async move {
        // `repeat` is infinite and always ready: without a yield between
        // items the task would never hit a pending await point, so disposal
        // could never take effect.
        let stream = stream::repeat(Ok::<_, &str>(111));
        let observable = FromTryStream::new(stream, scheduler.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_10_MS).await;
        assert!(!checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        // Let the abort take effect before sampling the count.
        scheduler.sleep(DURATION_10_MS).await;
        let count = checker.values().len();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values().len(), count);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
        let stream = rx;
        let observable = FromTryStream::new(stream, scheduler.clone());
        let (checker, observer) = Checker::new();

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Ok(111)).await.unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let source = stream::iter(vec![Ok::<_, &str>(111), Ok(222), Ok(333)]);

    let observable = FromTryStream::new(source, scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone()).take(1);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let pulled = Arc::new(AtomicUsize::new(0));
    let pulled_stream = pulled.clone();
    // Infinite and always ready: only the observer's answer can end it.
    let stream = stream::iter(1..).map(Ok::<_, &str>).inspect(move |_| {
        pulled_stream.fetch_add(1, Ordering::SeqCst);
    });
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::stopping_after(2);

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    // The observer ended its own stream on the second value, so the scheduler stops polling
    // the stream right there and the observer is dropped instead of being completed.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);

    time.advance_by(Duration::ZERO);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);
}

#[test]
fn test_stop_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let pulled = Arc::new(AtomicUsize::new(0));
    let pulled_stream = pulled.clone();
    let stream = stream::iter(1..).map(Ok::<_, &str>).inspect(move |_| {
        pulled_stream.fetch_add(1, Ordering::SeqCst);
    });
    let observable = FromTryStream::new(stream, scheduler.clone()).take(2);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    // `take` stops the source once it has its values, so the values behind them are never
    // pulled: only the completion of the operator itself reaches the observer.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);
}

#[test]
fn test_complete_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    tx.unbounded_send(Err("error")).unwrap();
    tx.unbounded_send(Ok(222)).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    time.advance_by(Duration::ZERO);
    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsub_after_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_after_error() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Err("error")).unwrap();
    time.advance_by(Duration::ZERO);
    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_order_with_continuous_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, &str>>();
    let stream = rx;
    let observable = FromTryStream::new(stream, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    let values = (0..1000).collect::<Vec<_>>();
    for i in &values {
        tx.unbounded_send(Ok(*i)).unwrap();
    }
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), values);
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), values);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let source = stream::iter(vec![Ok::<_, &str>(111), Ok(222), Ok(333)]);
    let observable = FromTryStream::new(source, scheduler.clone());
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let source = stream::iter(vec![Ok::<_, &str>(111), Ok(222), Ok(333)]);
    let observable = FromTryStream::new(source, scheduler.clone());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let source = stream::iter(vec![Ok::<_, &str>(111), Ok(222), Ok(333)]);
    let observable = FromTryStream::new(source, scheduler.clone());

    observable.filter(|_| true);
}
