mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::timer::Timer,
    subscription::disposable::Disposable,
};
use std::time::Duration;
use tests_utils::{checker::Checker, test_scheduler::TestScheduler};

#[test]
fn test_completed() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_unsubscribe() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        let _subscription_3 = observable_3.subscribe(observer_3);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        subscription_1.dispose();
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_dropped());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        subscription_2.dispose();
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_dropped());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_dropped());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_dropped());
        assert_eq!(checker_3.values(), [111]);
        assert!(checker_3.is_completed());
    });
}

#[test]
fn test_async() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        let (checker, observer) = Checker::new();

        let handle = spawn(async move { observable.subscribe(observer) });
        let _subscription = handle.await.unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
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
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_completed());
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_completed());
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
    });
}

#[test]
fn test_clone() {
    block_on(async {
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(async {
        // Custom operations
        let observable = Timer::new(111, Duration::from_millis(10), TestScheduler);

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);

        crate::tests_utils::test_runtime::sleep(Duration::from_millis(100)).await;
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(async {
        // Custom operations
        let observable = Timer::new(111, Duration::from_millis(100), TestScheduler);

        observable.filter(|_| true);
    });
}
