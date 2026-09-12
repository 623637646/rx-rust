mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    observer::{Flow, Termination},
    operators::creating::from_iter::FromIter,
};
use std::cell::Cell;
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed_array() {
    let source = [1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [1, 2, 3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_array_ref() {
    let source = [1, 2, 3];

    let observable = FromIter::new(&source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&1, &2, &3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_array_mut() {
    let mut source = [1, 2, 3];

    let observable = FromIter::new(&mut source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(source, [2, 4, 6]);
}

#[test]
fn test_completed_slice() {
    let source: &[i32] = &[1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&1, &2, &3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_slice_mut() {
    let mut data = [1, 2, 3];
    let source: &mut [i32] = &mut data;

    let observable = FromIter::new(source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(data, [2, 4, 6]);
}

#[test]
fn test_completed_vec() {
    let source = vec![1, 2, 3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [1, 2, 3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_vec_ref() {
    let source = vec![1, 2, 3];

    let observable = FromIter::new(&source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&1, &2, &3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_vec_mut() {
    let mut source = vec![1, 2, 3];

    let observable = FromIter::new(&mut source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(source, [2, 4, 6]);
}

#[test]
fn test_completed_range() {
    let source = 100..103;

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [100, 101, 102]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_ref() {
    let v1 = 1;
    let v2 = 2;
    let v3 = 3;
    let source = [&v1, &v2, &v3];

    let observable = FromIter::new(source);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&v1, &v2, &v3]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_mut_ref() {
    let mut v1 = 1;
    let mut v2 = 2;
    let mut v3 = 3;
    let source = [&mut v1, &mut v2, &mut v3];

    let observable = FromIter::new(source);

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| assert!(matches!(termination, Termination::Completed)),
    );
    assert_eq!(v1, 2);
    assert_eq!(v2, 4);
    assert_eq!(v3, 6);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let source = vec![1, 2, 3];
        let observable = FromIter::new(source);
        let (checker, observer) = Checker::<i32, Infallible>::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [1, 2, 3]);
        assert_eq!(checker.state(), State::Completed);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [1, 2, 3]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let source = [1, 2, 3];

    let observable = FromIter::new(source);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [1, 2, 3]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let source = [1, 2, 3];

    let observable = FromIter::new(source).take(1);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_stop_on_next() {
    let pulled = Cell::new(0);
    let observable = FromIter::new((1..).inspect(|_| pulled.set(pulled.get() + 1)));
    let (checker, observer) = Checker::<_, Infallible>::stopping_after(2);

    let _subscription = observable.subscribe(observer);
    // The observer ended its own stream on the second value, so the iterator is never pulled
    // again — an infinite one would otherwise never let `subscribe` return — and the observer is
    // dropped instead of being completed.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(pulled.get(), 2);
}

#[test]
fn test_stop_on_next_by_take() {
    let pulled = Cell::new(0);
    let observable = FromIter::new((1..).inspect(|_| pulled.set(pulled.get() + 1))).take(2);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    // `take` stops the source once it has its values, so the values behind them are never
    // produced: only the completion of the operator itself reaches the observer.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(pulled.get(), 2);
}

#[test]
fn test_stop_on_next_by_callback() {
    let pulled = Cell::new(0);
    let observable = FromIter::new((1..).inspect(|_| pulled.set(pulled.get() + 1)));
    let mut values = Vec::new();
    let mut terminated = false;

    // A plain callback can end its own stream by returning a `Flow`, which is what lets it break
    // out of an infinite synchronous source; `on_termination` is then never called.
    let _subscription = observable.subscribe_with_callback(
        |value| {
            values.push(value);
            if value == 2 {
                Flow::Stop
            } else {
                Flow::Continue
            }
        },
        |_| terminated = true,
    );
    assert_eq!(values, [1, 2]);
    assert!(!terminated);
    assert_eq!(pulled.get(), 2);
}

#[test]
fn test_clone() {
    let source = [1, 2, 3];
    let observable = FromIter::new(source);
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let source = [1, 2, 3];
    let observable = FromIter::new(source);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let source = [1, 2, 3];
    let observable = FromIter::new(source);

    observable.filter(|_| true);
}
