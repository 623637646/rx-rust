mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use futures::channel::oneshot::Canceled;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::{from_future::FromFuture, from_result::FromResult},
};
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable.map(|result| result.unwrap_or(-1));
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_drop() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel::<i32>();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable.map(|result| result.unwrap_or(-1));
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [-1]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let (_tx, rx) = futures::channel::oneshot::channel::<i32>();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable.map(|result| result.unwrap_or(-1));
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable.map(|result| result.unwrap_or(-1));
        let (checker, observer) = Checker::new();

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async move { tx.send(111).unwrap() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let source = std::future::ready(111);

        let observable = FromFuture::new(source, runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable.map(|result| result.unwrap_or(-1)).take(1);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable
            .map_infallible_to_error()
            .flat_map(FromResult::new);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).unwrap(); // completed after next
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_completed_drop() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel::<i32>();

        let observable = FromFuture::new(rx, runtime.clone());
        let observable = observable
            .map_infallible_to_error()
            .flat_map(FromResult::new);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error(Canceled));
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let source = std::future::ready(111);
        let observable = FromFuture::new(source, runtime.clone());
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let source = async { 111 };
        let observable = FromFuture::new(source, runtime.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let source = async { 111 };
        let observable = FromFuture::new(source, runtime.clone());

        observable.filter(|_| true);
    });
}
