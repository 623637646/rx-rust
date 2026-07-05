mod tests_utils;

use crate::tests_utils::DURATION_3_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::timer::Timer,
};
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());
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
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        subscription_1.dispose();
        runtime.sleep(DURATION_3_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        subscription_2.dispose();
        runtime.sleep(DURATION_3_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Dropped);
        assert!(checker_3.values().is_empty());
        assert_eq!(checker_3.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Dropped);
        assert_eq!(checker_3.values(), [111]);
        assert_eq!(checker_3.state(), State::Completed);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());
        let (checker, observer) = Checker::new();

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone()).take(1);
        let (checker, observer) = Checker::new();

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());
        _ = observable.clone();
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);

        runtime.sleep(DURATION_100_MS).await;
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Timer::new(111, DURATION_100_MS, runtime.clone());

        observable.filter(|_| true);
    });
}
