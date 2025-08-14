mod tests_utils;

use crate::tests_utils::DURATION_NEXT_LOOP;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, observable_ext::ObservableExt},
    operators::creating::from_result::FromResult,
};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let observable = FromResult::<_, Infallible>::new(Ok(111));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let observable = FromResult::<i32, _>::new(Err("error"));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;

    let observable = FromResult::<_, Infallible>::new(Ok(&value));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let observable = FromResult::<_, Infallible>::new(Ok(&mut value));
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_termination,
    );

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let observable = FromResult::<_, Infallible>::new(Ok(111));
        let (checker, observer) = Checker::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_NEXT_LOOP).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = FromResult::<_, Infallible>::new(Ok(111));
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let observable = FromResult::<_, Infallible>::new(Ok(111)).take(0);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_clone() {
    let observable = FromResult::<_, Infallible>::new(Ok(111));
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = FromResult::<_, Infallible>::new(Ok(111));

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = FromResult::<_, Infallible>::new(Ok(111));

    observable.filter(|_| true);
}
