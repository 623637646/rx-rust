mod tests_utils;

use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::subject::Subject;
use rx_rust::subject::replay_subject::ReplaySubject;
use rx_rust::subscription::Subscription;
use rx_rust::subscription::disposable::Disposable;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_completed());
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [333]);
    assert!(checker.is_completed());
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_error("error"));
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [333]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_error("error"));
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(333);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [222, 333]);
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_error("error"));
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    let _subscription_3 = observable_3.subscribe(observer_3);
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_completed());
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
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(&value_1);
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let _subscription_2 = observable.subscribe(observer_2);
    assert_eq!(checker_1.values(), [&value_1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [&value_1]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(&value_2);
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(&value_3);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [&value_1, &value_2]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [&value_1, &value_2]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert!(checker.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[tokio::test]
async fn test_async() {
    let subject = ReplaySubject::new(None);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let observable_cloned = observable.clone();
    let handle = tokio::spawn(async move { observable_cloned.subscribe(observer_1) });
    let _subscription_1 = handle.await.unwrap();
    let subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let handle = tokio::spawn(async move { observable.subscribe(observer_2) });
    let _subscription_2 = handle.await.unwrap();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Completed);
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(333);
    });
    handle.await.unwrap();
    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move { subject_cloned.subscribe(observer) });
    let _subscription = handle.await.unwrap();
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
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
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_complete_on_next() {
    let mut subject = ReplaySubject::new(None);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = Some(subject.clone());
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .take()
                .unwrap()
                .on_termination(Termination::<Infallible>::Completed);
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error_on_next() {
    let mut subject = ReplaySubject::new(None);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = Some(subject.clone());
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .take()
                .unwrap()
                .on_termination(Termination::Error("error"));
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
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
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .clone()
            .hook_on_next(move |value, callback| {
                if let Some(sub) = { sub_cloned.lock().unwrap().take() } {
                    sub.dispose();
                }
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .hook_on_next(move |value, callback| {
                callback(value);
                if let Some(sub) = { sub_cloned.lock().unwrap().take() } {
                    sub.dispose();
                }
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), []);
    assert!(checker_3.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_dropped());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_dropped());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_dropped());
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
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .clone()
            .hook_on_termination(move |value, callback| {
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .hook_on_termination(move |value, callback| {
                callback(value);
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), []);
    assert!(checker_3.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_completed());
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
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .clone()
            .hook_on_termination(move |value, callback| {
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
                callback(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Arc::new(Mutex::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock().unwrap() = Some(
        observable
            .hook_on_termination(move |value, callback| {
                callback(value);
                { sub_cloned.lock().unwrap().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), []);
    assert!(checker_3.is_active());
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
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
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: ReplaySubject<'_, i32, String> = ReplaySubject::new(None);
    let observable = subject;

    observable.filter(|_| true);
}
