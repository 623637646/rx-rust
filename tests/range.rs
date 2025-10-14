mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::range::Range,
};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed_range() {
    let source = 100..103;
    let observable = Range::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100, 101, 102]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_range_inclusive() {
    let source = 100..=103;
    let observable = Range::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100, 101, 102, 103]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let source = 100..103;
        let observable = Range::new(source);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [100, 101, 102]);
        assert_eq!(checker.state(), State::Completed);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [100, 101, 102]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let source = 100..103;
    let observable = Range::new(source);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [100, 101, 102]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [100, 101, 102]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let source = 100..103;
    let observable = Range::new(source).take(1);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_clone() {
    let source = 100..103;
    let observable = Range::new(source);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = 100..103;
    let observable = Range::new(source);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = 100..103;
    let observable = Range::new(source);

    observable.filter(|_| true);
}
