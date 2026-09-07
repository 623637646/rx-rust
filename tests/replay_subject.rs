mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::observable::Observable;
use rx_rust::observable::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::safe_lock;
use rx_rust::safe_lock_option;
use rx_rust::safe_lock_option_disposable;
use rx_rust::safe_lock_vec;
use rx_rust::subject::Subject;
use rx_rust::subject::replay_subject::ReplaySubject;
use rx_rust::utils::types::{Mutable, Shared};
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_completed_with_buffer_0() {
    let mut subject = ReplaySubject::new(Some(0));
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_completed_with_buffer_1() {
    let mut subject = ReplaySubject::new(Some(1));
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [333]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination. The buffer is replayed before the error.
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_error_with_buffer_0() {
    let mut subject = ReplaySubject::new(Some(0));
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_error_with_buffer_1() {
    let mut subject = ReplaySubject::new(Some(1));
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination. The buffer is replayed before the error.
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [333]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsubscribe() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();
    let observable_3 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    let _subscription_3 = observable_3.subscribe(observer_3);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;

    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(&value_1);
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [&value_1]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(&value_2);
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(&value_3);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = ReplaySubject::new(None);
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let observable_cloned = observable.clone();
        let _subscription_1 = runtime
            .spawn(async move { observable_cloned.subscribe(observer_1) })
            .await
            .unwrap();
        let subject_cloned = subject.clone();
        let _subscription = observable.clone().subscribe_with_callback(
            |_| {},
            move |_| {
                // Terminate Subject itself first. Then terminate Observers in Subject.
                assert!(subject_cloned.terminated().is_some());
            },
        );
        assert_eq!(checker_1.values(), []);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), []);
        assert_eq!(checker_2.state(), State::Active);
        assert!(subject.terminated().is_none());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), []);
        assert_eq!(checker_2.state(), State::Active);
        assert!(subject.terminated().is_none());

        let _subscription_2 = runtime
            .spawn(async move { observable.subscribe(observer_2) })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert!(subject.terminated().is_none());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);
        assert!(subject.terminated().is_none());

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Completed);
        assert!(matches!(subject.terminated(), Some(Termination::Completed)));

        // on_next and on_termination after termination
        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(333);
            })
            .await
            .unwrap();
        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Completed);
        assert!(matches!(subject.terminated(), Some(Termination::Completed)));

        // subscribe after termination
        let (checker, observer) = Checker::new();
        let subject_cloned = subject.clone();
        let _subscription = runtime
            .spawn(async move { subject_cloned.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_next_by_take() {
    let mut subject = ReplaySubject::<'_, _, Infallible>::new(None);
    let (checker_1, observer_1) = Checker::new();

    // Custom operations
    let observable = subject.clone().take(1);

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // In this case, Subject is not terminated.
            assert!(subject_cloned.terminated().is_none());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(subject.terminated().is_none());
}

#[test]
fn test_complete_on_next() {
    let mut subject = ReplaySubject::new(None);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .clone()
                .on_termination(Termination::<Infallible>::Completed);
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error_on_next() {
    let mut subject = ReplaySubject::new(None);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .clone()
                .on_termination(Termination::Error("error"));
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsub_on_next() {
    let mut subject: ReplaySubject<'_, _, Infallible> = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_next(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_sub_on_next() {
    let mut subject: ReplaySubject<'_, _, Infallible> = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                // subscribe before on_next
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_next(value);
                // subscribe after on_next
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
}

#[test]
fn test_next_on_next() {
    let mut subject = ReplaySubject::new(None);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            // The subject is terminated as soon as `on_termination` is called, which the callback
            // of `1` did: the callback of `2` sees it terminated while the termination is still
            // queued behind that value.
            assert_eq!(subject_cloned.terminated().is_some(), value > 1);
            if value < 3 {
                subject_cloned.on_next(value + 1);
            }
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(1);
    // The events are delivered in the order they were sent: `3` is sent from the callback of `2`,
    // after that callback has already sent the termination, so it arrives once the subject has
    // terminated and is dropped.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_completed() {
    let mut subject: ReplaySubject<'_, _, Infallible> = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_sub_on_completed() {
    let mut subject: ReplaySubject<'_, _, Infallible> = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_error() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_sub_on_error() {
    let mut subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                // subscribe before termination
                if let Some(observer) = observer_2.take() {
                    safe_lock_option!(replace: subscription_2_cloned, observable.clone().subscribe(observer));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    safe_lock_option!(replace: subscription_3_cloned, observable.clone().subscribe(observer));
                }
            })
            .subscribe(observer_1),
    );
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Error("error"));
    // Both subscribed once the subject was terminated, so both get the buffer and then the error.
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

// The cases below assert the behaviour `ReplaySubject` is meant to have. It keeps its buffer in a
// lock of its own, next to the lock of the `PublishSubject`'s delivery, and nothing is atomic
// across the two, so they fail today: each of them names the window it walks through.

#[test]
fn test_next_on_sub() {
    // `subscribe` replays a snapshot of the buffer under one lock and joins the subject under the
    // other, and the callbacks of the replayed values run in between: the `222` sent from there is
    // buffered and forwarded before the observer is admitted, so it reaches neither the replay nor
    // the observer.
    let mut subject: ReplaySubject<'_, _, Infallible> = ReplaySubject::new(None);
    subject.on_next(111);

    // Custom operations
    let observable = subject.clone();

    let mut subject_cloned = subject.clone();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        move |value| {
            safe_lock_vec!(push: values_cloned, value);
            if value == 111 {
                subject_cloned.on_next(222);
            }
        },
        move |_| {},
    );

    // The observer had joined the subject before `222` was sent, so it sees it too, and it sees
    // what a subscriber arriving afterwards is replayed.
    assert_eq!(safe_lock!(clone: values), [111, 222]);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
}

#[cfg(not(feature = "single-threaded"))]
#[test]
fn test_race_condition() {
    // Same window as `test_next_on_sub`, walked into by another thread: a value buffered between
    // the snapshot and the subscription is replayed to the observer and forwarded to it as well,
    // and a value forwarded there reaches neither.
    use std::sync::{Arc, Barrier};

    const SUBSCRIBERS: usize = 3;
    const ROUNDS: usize = 20;

    for _ in 0..200 {
        let mut subject: ReplaySubject<'static, i32, Infallible> = ReplaySubject::new(None);
        let barrier = Arc::new(Barrier::new(SUBSCRIBERS + 1));

        let handles: Vec<_> = (0..SUBSCRIBERS)
            .map(|_| {
                let subject = subject.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    // The subscriptions are returned so that the observers stay subscribed until
                    // every thread has finished.
                    (0..ROUNDS)
                        .map(|_| {
                            let (checker, observer) = Checker::new();
                            (checker, subject.clone().subscribe(observer))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        barrier.wait();
        subject.on_next(111);

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        for (checker, _subscription) in results.iter().flatten() {
            // The observer subscribed either before `111` or after it, so it is either forwarded
            // the value or replayed it, exactly once in both cases.
            assert_eq!(checker.values(), [111]);
            assert_eq!(checker.state(), State::Active);
        }
    }
}

#[cfg(not(feature = "single-threaded"))]
#[test]
fn test_race_condition_between_next() {
    // Buffering a value and forwarding it are two steps under two locks, so two threads can buffer
    // in one order and forward in the other: what a late subscriber is replayed is then not what
    // the subject delivered.
    use std::sync::{Arc, Barrier};

    const SENDERS: usize = 16;

    for _ in 0..500 {
        let subject: ReplaySubject<'static, i32, Infallible> = ReplaySubject::new(None);
        let (checker, observer) = Checker::new();
        let _subscription = subject.clone().subscribe(observer);
        let barrier = Arc::new(Barrier::new(SENDERS));

        let handles: Vec<_> = (1..=SENDERS)
            .map(|value| {
                let mut subject = subject.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    subject.on_next(value as i32);
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }

        let (late_checker, late_observer) = Checker::new();
        let _subscription = subject.subscribe(late_observer);
        assert_eq!(
            late_checker.values(),
            checker.values(),
            "the replayed values are not the values the subject delivered"
        );
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker));
        let subject = ReplaySubject::new(None);
        _subscription = subject.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = ReplaySubject::<'_, TestStruct, TestStruct>::new(None);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: ReplaySubject<'_, i32, String> = ReplaySubject::new(None);
    let observable = subject;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: ReplaySubject<'_, i32, String> = ReplaySubject::new(None);
    let observable = subject;

    observable.filter(|_| true);
}
