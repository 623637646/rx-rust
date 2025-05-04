mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{
        combining::merge_all::MergeAll,
        creating::{create::Create, just::Just, throw::Throw},
    },
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_outer_finish() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_empty() {
    let subject: PublishSubject<'_, Just<i32>, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_same_inner() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_1.on_next(222);
    assert!(checker.is_values_matched(&[111, 222, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 222, 333, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 222, 333, 333]));
    assert!(checker.is_completed());
}

#[test]
fn test_completed_unsubscribe() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subscription.unsubscribe();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());
}

#[test]
fn test_error_inner_finish() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_outer_finish() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_empty() {
    let subject: PublishSubject<'_, Throw<_>, _> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_same_inner() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_1.on_next(222);
    assert!(checker.is_values_matched(&[111, 222, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 222, 333, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222, 222, 333, 333]));
    assert!(checker.is_error("error"));
}

#[test]
fn test_error_unsubscribe() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subscription.unsubscribe();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_1.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    subject_2.on_termination(Termination::Error("error"));
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject_1.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject_2.on_next(222);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();

    subject_1.on_next(333);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222, 333]));
    assert!(checker_2.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222, 333]));
    assert!(checker_2.is_active());

    subject_2.on_next(444);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_2.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_2.is_completed());
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let value_4 = 333;
    let error = -1;

    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(&value_1);
    assert!(checker.is_values_matched(&[&value_1]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[&value_1]));
    assert!(checker.is_active());

    subject_2.on_next(&value_2);
    assert!(checker.is_values_matched(&[&value_1, &value_2]));
    assert!(checker.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[&value_1, &value_2]));
    assert!(checker.is_active());

    subject_1.on_next(&value_3);
    assert!(checker.is_values_matched(&[&value_1, &value_2, &value_3]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[&value_1, &value_2, &value_3]));
    assert!(checker.is_active());

    subject_2.on_next(&value_4);
    assert!(checker.is_values_matched(&[&value_1, &value_2, &value_3, &value_4]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Error(&error));
    assert!(checker.is_values_matched(&[&value_1, &value_2, &value_3, &value_4]));
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;
    let mut error = -1;

    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(Just::new(&mut value_1).map_infallible_to_error());
        observer.on_next(Just::new(&mut value_2).map_infallible_to_error());
        observer.on_next(Just::new(&mut value_3).map_infallible_to_error());
        observer.on_termination(Termination::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let observable = observable.merge_all();

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => *error *= 2,
        },
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 666);
    assert_eq!(error, -2);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let subject_1 = PublishSubject::default();
    let subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();

    let handle = tokio::spawn(async move { observable.subscribe(observer) });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let subject_1_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(subject_1_cloned.clone());
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let subject_2_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(subject_2_cloned.clone());
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    let mut subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(222);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    let mut subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(333);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    let subject_cloned = subject_1.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Completed);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    let mut subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(444);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());

    let subject_cloned = subject_2.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject_1.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject_2.on_next(222);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[111, 222]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject_1.on_next(333);
    assert!(checker_1.is_values_matched(&[111, 222, 333]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222, 333]));
    assert!(checker_2.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker_1.is_values_matched(&[111, 222, 333]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222, 333]));
    assert!(checker_2.is_active());

    subject_2.on_next(444);
    assert!(checker_1.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_2.is_active());

    subject_2.on_termination(Termination::Error("error"));
    assert!(checker_1.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let mut subject_3 = PublishSubject::default();
    let mut subject_4 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.merge_all().merge_all();

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(subject_3.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_2.on_next(subject_4.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_3.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_4.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_3.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_4.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let mut subject_2 = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = MergeAll::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(subject_1.clone());
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject_1.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_next(subject_2.clone());
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject_2.on_next(222);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert!(checker.is_values_matched(&[111, 222]));
    assert!(checker.is_active());

    subject_1.on_next(333);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_1.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333]));
    assert!(checker.is_active());

    subject_2.on_next(444);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_active());

    subject_2.on_termination(Termination::Completed);
    assert!(checker.is_values_matched(&[111, 222, 333, 444]));
    assert!(checker.is_completed());
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(Just::new(1));
            observer.on_termination(Termination::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });

        let observable = observable.merge_all();

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(
            |observer: BoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_1 = Some(observer);
                Subscription::new_none_disposal()
            },
        );
        let observable = observable.merge_all();

        let (_, mut observer) = Checker::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub = TestStruct;
    let mut life_marker_or = None;

    // Error
    // let mut life_marker_or = None;
    // let life_marker_sub = TestStruct;

    {
        let observable = Create::new(
            |observer: BoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_or = Some(observer);
                Subscription::new_with_disposal_callback(|| {
                    life_marker_sub.consume_ref();
                })
            },
        );

        let observable = observable.merge_all();

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(Just::new(TestStruct).map_infallible_to_error());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.merge_all();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, _> = PublishSubject::default();
    let observable = subject.merge_all();

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, Infallible> = PublishSubject::default();
    let observable: MergeAll<_, Just<i32>> = subject.merge_all();

    observable.buffer_with_count(1);
}
