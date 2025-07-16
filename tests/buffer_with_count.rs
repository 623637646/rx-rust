mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::buffer_with_count::BufferWithCount},
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(666);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_count_1() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(1).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(666);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_count_1() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(1).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(222);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_3);
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value_1);
        observer.on_next(&mut value_2);
        observer.on_next(&mut value_3);
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.subscribe_with_callback(
        |value| {
            for i in value {
                *i *= 2;
            }
        },
        |termination| assert!(matches!(termination, Termination::Error("error"))),
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 333);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

        let subscription = runtime
            .clone()
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(333);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .clone()
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .buffer_with_count(NonZeroUsize::new(2).unwrap())
        .buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(666);
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(777);
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            vec![vec![111, 222], vec![333, 444]],
            vec![vec![555, 666], vec![777]]
        ]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = BufferWithCount::new(observable, NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(222);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(333);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(444);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(555);
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker.state(), State::Completed);
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
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable
            .buffer_with_count(NonZeroUsize::new(2).unwrap())
            .buffer_with_count(NonZeroUsize::new(2).unwrap());

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
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(vec![&life_marker_2]);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(NonZeroUsize::new(3).unwrap());

    observable.filter(|_| true);
}
