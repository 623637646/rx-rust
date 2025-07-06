mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::interval::Interval,
    subscription::disposable::Disposable,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};

#[test]
fn test_completed_no_delay() {
    block_on(async {
        let observable = Interval::new(Duration::from_millis(100), TestScheduler, None);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [0]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_active());

        subscription.dispose();
        assert_eq!(checker.values(), [0, 1, 2]);
        // assert!(checker.is_active()); // This assert may be failed in multi-thread.

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_completed_with_delay() {
    block_on(async {
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_active());

        subscription.dispose();
        assert_eq!(checker.values(), [0, 1, 2]);
        // assert!(checker.is_active()); // This assert may be failed in multi-thread.

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_unsubscribe() {
    block_on(async {
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

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0, 1]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert!(checker_2.is_active());

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [0, 1, 2]);
        // assert!(checker_1.is_active()); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [0, 1, 2, 3]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [0, 1, 2, 3, 4]);
        assert!(checker_2.is_active());
    });
}

#[test]
fn test_async() {
    block_on(async {
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let subscription = spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_active());

        spawn(async { subscription.dispose() }).await.unwrap();
        assert_eq!(checker.values(), [0, 1, 2]);
        // assert!(checker.is_dropped()); // This assert may be failed in multi-thread.

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(async {
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

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0, 1]);
        assert!(checker_2.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert!(checker_2.is_active());

        subscription_1.dispose();
        subscription_2.dispose();
        assert_eq!(checker_1.values(), [0, 1, 2]);
        // assert!(checker_1.is_active()); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [0, 1, 2]);
        // assert!(checker_2.is_active()); // This assert may be failed in multi-thread.

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert!(checker_2.is_dropped());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [0, 1, 2]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [0, 1, 2]);
        assert!(checker_2.is_dropped());
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(async {
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
    });
}

#[test]
fn test_clone() {
    block_on(async {
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(async {
        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(async {
        // Custom operations
        let observable = Interval::new(
            Duration::from_millis(100),
            TestScheduler,
            Some(Duration::from_millis(100)),
        );

        observable.filter(|_| true);
    });
}
