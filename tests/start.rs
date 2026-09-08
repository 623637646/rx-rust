mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::observable::{Observable, ObservableExt};
use rx_rust::operators::creating::start::Start;
use rx_rust::utils::mutable::{MutableBool, MutableBoolHelper};
use rx_rust::utils::types::Shared;
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let (checker, observer) = Checker::new();
    let called = MutableBool::new(false);

    // Custom operations
    let observable = Start::new(|| {
        called.write(true);
        111
    });

    assert!(!called.read());
    let _subscription = observable.subscribe(observer);
    assert!(called.read());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value = 111;
    let (checker, observer) = Checker::new();
    let called = MutableBool::new(false);

    // Custom operations
    let observable = Start::new(|| {
        called.write(true);
        &value
    });

    assert!(!called.read());
    let _subscription = observable.subscribe(observer);
    assert!(called.read());
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let (checker, observer) = Checker::new();
    let called = MutableBool::new(false);

    // Custom operations
    let observable = Start::new(|| {
        called.write(true);
        &mut value
    });

    assert!(!called.read());
    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_termination,
    );
    assert!(called.read());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();
        let called = Shared::new(MutableBool::new(false));
        let called_cloned = called.clone();
        // Custom operations
        let observable = Start::new(move || {
            called_cloned.write(true);
            111
        });

        assert!(!called.read());
        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(called.read());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let called = MutableBool::new(false);

    // Custom operations
    let observable = Start::new(|| {
        called.write(true);
        111
    });
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    assert!(!called.read());
    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(called.read());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (checker, observer) = Checker::new();
    let called = MutableBool::new(false);

    // Custom operations
    let observable = Start::new(|| {
        called.write(true);
        111
    })
    .take(1);

    assert!(!called.read());
    let _subscription = observable.subscribe(observer);
    assert!(called.read());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_clone() {
    let observable = Start::new(|| 111);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Start::new(|| 111);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Start::new(|| 111);

    observable.filter(|_| true);
}
