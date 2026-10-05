mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_scheduler::block_on;
use futures::FutureExt;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    operators::creating::from_try_future::FromTryFuture,
};
use tests_utils::checker::Checker;

/// A one-shot channel whose receiver is a `Future<Output = Result<i32, &str>>`: dropping the
/// sender fails it with `"canceled"`.
#[allow(clippy::type_complexity)]
fn try_channel() -> (
    futures::channel::oneshot::Sender<Result<i32, &'static str>>,
    impl Future<Output = Result<i32, &'static str>>,
) {
    let (tx, rx) = futures::channel::oneshot::channel();
    (tx, rx.map(|result| result.unwrap_or(Err("canceled"))))
}

#[test]
fn test_completed() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Ok(111)).unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Err("error")).unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_error_drop() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("canceled"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|scheduler| async move {
        let (_tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler
            .spawn(async move { tx.send(Ok(111)).unwrap() })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|scheduler| async move {
        let source = std::future::ready(Ok::<_, &str>(111));

        let observable = FromTryFuture::new(source, scheduler.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone()).take(1);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Ok(111)).unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_stop_on_next() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::stopping_after(1);

        let _subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Ok(111)).unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        // The observer ended its own stream on the value, so it is dropped instead of being
        // completed.
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Ok(111)).unwrap(); // completed after next
        scheduler.sleep(DURATION_10_MS).await;
        subscription.dispose();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|scheduler| async move {
        let (tx, rx) = try_channel();

        let observable = FromTryFuture::new(rx, scheduler.clone());
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_10_MS).await; // make sure it's subscribed
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(Err("error")).unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_clone() {
    block_on(|scheduler| async move {
        let source = std::future::ready(Ok::<_, &str>(111));
        let observable = FromTryFuture::new(source, scheduler.clone());
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|scheduler| async move {
        // Custom operations
        let source = async { Ok::<_, &str>(111) };
        let observable = FromTryFuture::new(source, scheduler.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|scheduler| async move {
        // Custom operations
        let source = async { Ok::<_, &str>(111) };
        let observable = FromTryFuture::new(source, scheduler.clone());

        observable.filter(|_| true);
    });
}
