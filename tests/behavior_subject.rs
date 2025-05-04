mod tests_utils;

use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Terminal};
use rx_rust::subject::behavior_subject::BehaviorSubject;
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[-1]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert!(checker.is_values_matched(&[-1, 111]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[-1, 111]));
    assert!(checker.is_completed());
    assert!(matches!(subject.terminated(), Some(Terminal::Completed)));
    assert_eq!(subject.value(), 111);

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_error() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[-1]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert!(checker.is_values_matched(&[-1, 111]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[-1, 111]));
    assert!(checker.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_unsubscribe() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[-1]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[-1]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[-1, 111]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[-1, 111]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[-1, 111, 222]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 222);

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[-1, 111, 222]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 222);

    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_ref() {
    let value_1 = -1;
    let value_2 = 111;
    let error = 222;

    let mut subject = BehaviorSubject::new(&value_1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[&value_1]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &value_1);

    subject.on_next(&value_2);
    assert!(checker.is_values_matched(&[&value_1, &value_2]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &value_2);

    subject.clone().on_terminal(Terminal::Error(&error));
    assert!(checker.is_values_matched(&[&value_1, &value_2]));
    assert!(checker.is_error(&error));
    assert!(matches!(subject.terminated(), Some(Terminal::Error(&222))));
    assert_eq!(subject.value(), &value_2);

    _ = subscription; // keep the subscription alive
}

#[tokio::test]
async fn test_async() {
    let subject = BehaviorSubject::new(&-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[&-1]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &-1);

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(&111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&-1, &111]));
    assert!(checker.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &111);

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&-1, &111]));
    assert!(checker.is_dropped());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &111);

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&-1, &111]));
    assert!(checker.is_dropped());
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), &111);
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[-1]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[-1]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[-1, 111]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[-1, 111]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    _ = subscription_1; // keep the subscription alive
    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let subscription;

    // Error
    // let subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker));
        let subject = BehaviorSubject::new(None);
        subscription = subject.subscribe(observer);
    }

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_clone() {
    let observable = BehaviorSubject::<'_, _, TestStruct>::new(TestStruct);
    let _ = observable.clone(); // make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_actions_after_terminal() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let subscription_1 = subject.clone().subscribe(observer_1);
    assert!(checker_1.is_values_matched(&[-1]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    let subscription_2 = subject.clone().subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::Completed);
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    subject.clone().on_terminal(Terminal::Error("error2"));
    assert!(checker_1.is_values_matched(&[-1, 111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Terminal::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    _ = subscription_1; // keep the subscription alive
    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: BehaviorSubject<'_, i32, String> = BehaviorSubject::new(-1);
    let observable = subject;

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: BehaviorSubject<'_, i32, String> = BehaviorSubject::new(-1);
    let observable = subject;

    let _ = observable.buffer_with_count(1);
}
