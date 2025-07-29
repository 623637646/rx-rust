mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::{
    RECURSION_EXECUTION_TIMES, RECURSION_EXPECTED_DIFF, RECURSION_SLEEP_TIME,
};
use futures::StreamExt;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::safe_lock::{SafeLock, SafeLockOption};
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::interval::Interval,
};
use std::time::{Duration, Instant};
use tests_utils::checker::Checker;

#[test]
fn test_completed_no_delay() {
    block_on(|runtime| async move {
        let observable = Interval::new(Duration::from_millis(100), runtime.clone(), None);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        assert_eq!(checker.values(), [0, 1, 2]);
        // assert_eq!(checker.state(), State::Active); // This assert may be failed in multi-thread.

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_completed_with_delay() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        assert_eq!(checker.values(), [0, 1, 2]);
        // assert_eq!(checker.state(), State::Active); // This assert may be failed in multi-thread.

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0, 1]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert_eq!(checker_2.state(), State::Active);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [0, 1, 2]);
        // assert_eq!(checker_1.state(), State::Active); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [0, 1, 2, 3]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [0, 1, 2, 3, 4]);
        assert_eq!(checker_2.state(), State::Active);
    });
}

#[test]
fn test_precision() {
    block_on(|runtime| async move {
        let start_instant = Instant::now();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let observable = Interval::new(
            Duration::from_millis(RECURSION_SLEEP_TIME),
            runtime.clone(),
            None,
        );
        let _subscription = observable.subscribe_with_callback(
            move |_| {
                tx.unbounded_send(Instant::now()).unwrap();
            },
            |_| {},
        );

        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff =
                duration.as_micros() - (count * RECURSION_SLEEP_TIME as usize * 1000) as u128;
            assert!(
                diff < RECURSION_EXPECTED_DIFF,
                "diff: {diff}, count: {count}"
            );
            count += 1;
            if count == RECURSION_EXECUTION_TIMES {
                break;
            }
        }
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0, 1]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert_eq!(checker_2.state(), State::Active);

        subscription_1.dispose();
        subscription_2.dispose();
        assert_eq!(checker_1.values(), [0, 1, 2]);
        // assert_eq!(checker_1.state(), State::Active); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [0, 1, 2]);
        // assert_eq!(checker_2.state(), State::Active); // This assert may be failed in multi-thread.

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert_eq!(checker_2.state(), State::Dropped);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert_eq!(checker_2.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        )
        .take(1);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = Shared::new(Mutable::new(None::<Subscription<'_>>));
        let subscription_cloned = subscription.clone();
        let (mut on_next, on_termination) = observer.into_callbacks();
        subscription.safe_lock_set(Some(observable.subscribe_with_callback(
            move |value| {
                on_next(value);
                subscription_cloned.safe_lock_take().unwrap().dispose();
            },
            |termination| {
                on_termination(termination);
            },
        )));
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert!(subscription.safe_lock_is_some());

        runtime.sleep(Duration::from_millis(110)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Dropped);
        assert!(subscription.safe_lock_is_none());
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        observable.filter(|_| true);
    });
}
