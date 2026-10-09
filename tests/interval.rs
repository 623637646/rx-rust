mod tests_utils;

use crate::tests_utils::DURATION_1_MS;
use crate::tests_utils::DURATION_1_YEAR;
use crate::tests_utils::DURATION_3_MS;
use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableExt},
    operators::creating::interval::Interval,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tests_utils::checker::Checker;

#[test]
fn test_completed_zero_initial_delay() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable =
        Interval::with_initial_delay(Duration::ZERO, DURATION_100_MS, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0, 1]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(DURATION_3_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Dropped);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Dropped);
}

/// The first value comes one period after the subscription, as in ReactiveX.
#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [0]);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [0, 1]);
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(DURATION_3_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Dropped);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Dropped);
}

/// An initial delay other than the period: the first value comes after it, the next ones a period
/// apart from there.
#[test]
fn test_completed_with_initial_delay() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable =
        Interval::with_initial_delay(DURATION_30_MS, DURATION_100_MS, scheduler.clone());
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(DURATION_30_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [0]);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), [0]);

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [0, 1]);
    assert_eq!(checker.state(), State::Active);
}

/// An initial delay too long for an `Instant` to represent never ends: the first value never comes,
/// and the task waits until it is disposed.
#[test]
fn test_initial_delay_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable =
        Interval::with_initial_delay(Duration::MAX, DURATION_100_MS, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(DURATION_1_YEAR);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(time.pending_tasks(), 1);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

/// A period too long for an `Instant` to represent: the second value never comes, so the task
/// finishes after the first one and drops the observer without a termination, as `Never` does
/// (decision 0004).
#[test]
fn test_period_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::with_initial_delay(Duration::ZERO, Duration::MAX, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);

    time.advance_by(DURATION_1_YEAR);
    subscription.dispose();
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = Interval::new(DURATION_100_MS, scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 1]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2, 3]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2, 3, 4]);
    assert_eq!(checker_2.state(), State::Active);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        // Half the durations of the other tests: this one runs on real time, on every scheduler.
        let observable = Interval::new(DURATION_100_MS / 2, scheduler.clone());
        let (checker, observer) = Checker::new();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_30_MS / 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_100_MS / 2).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_100_MS / 2).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_100_MS / 2).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS / 2).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        scheduler.sleep(DURATION_100_MS / 2).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        scheduler.sleep(DURATION_100_MS / 2).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = Interval::new(DURATION_100_MS, scheduler.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 1]);
    assert_eq!(checker_2.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    subscription_2.dispose();
    time.advance_by(DURATION_3_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Dropped);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Dropped);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker_1.values(), [0, 1, 2]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 1, 2]);
    assert_eq!(checker_2.state(), State::Dropped);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone()).take(1);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    time.advance_by(DURATION_100_MS);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone());
    let (checker, observer) = Checker::new();

    let subscription = Arc::new(Mutex::new(None));
    let subscription_cloned = subscription.clone();
    let (mut on_next, on_termination) = observer.into_callbacks();
    subscription.replace_value(Some(observable.subscribe_with_callback(
        move |value| {
            on_next(value);
            if let Some(subscription) = subscription_cloned.take_value() {
                Disposable::dispose(subscription);
            }
        },
        |termination| {
            on_termination(termination);
        },
    )));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subscription.with_ref(Option::is_some));

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subscription.with_ref(Option::is_some));

    time.advance_by(DURATION_1_MS);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(subscription.with_ref(Option::is_none));
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone());
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let observable = Interval::new(DURATION_100_MS, scheduler.clone());

    observable.filter(|_| true);
}
