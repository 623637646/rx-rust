mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    operators::creating::never::Never,
};
use tests_utils::checker::Checker;

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let observable = Never;
        let (checker, observer) = Checker::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Never;
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
}

#[test]
fn test_unsub_on_next_by_take() {
    let observable = Never.take(0);
    let (checker, observer) = Checker::new();

    // Custom operations
    let _subscription = observable.subscribe(observer);

    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_clone() {
    let observable = Never;
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Never;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Never;

    observable.filter(|_| true);
}
