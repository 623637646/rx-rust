mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::group_by::GroupBy},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let mut index = -1;
    let subscription_1 = observable_1
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_1);

    let mut index = -1;
    let _subscription_2 = observable_2
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_2);

    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19, 121]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19, 121]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = 444;

    let mut subject: PublishSubject<'_, &i32, &i32> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| *value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| (i, v))
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(&value_1);
    assert_eq!(checker.values(), [(0, &value_1)]);
    assert!(checker.is_active());

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [(0, &value_1), (1, &value_2)]);
    assert!(checker.is_active());

    subject.on_next(&value_3);
    assert_eq!(
        checker.values(),
        [(0, &value_1), (1, &value_2), (0, &value_3)]
    );
    assert!(checker.is_active());

    subject.on_termination(Termination::Error(&error));
    assert_eq!(
        checker.values(),
        [(0, &value_1), (1, &value_2), (0, &value_3)]
    );
    assert!(checker.is_error(&error));
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);

    let handle = tokio::spawn(async {
        let mut index = -1;
        observable
            .flat_map(move |v| {
                index += 1;
                let i = index;
                v.map(move |v| 10 * i + v)
            })
            .subscribe(observer)
    });
    let subscription = handle.await.unwrap();
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        for i in 0..=9 {
            subject_cloned.on_next(i);
        }
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let mut index_1 = -1;
    let mut index_2 = -1;
    let _subscription_1 = observable_1
        .flat_map(|v| {
            index_1 += 1;
            let i = index_1;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2
        .flat_map(|v| {
            index_2 += 1;
            let i = index_2;
            v.map(move |v| 10 * i + v)
        })
        .subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let mut index = -1;
    let observable = observable.group_by(|value| value % 4).group_by(|_| {
        index += 1;
        index % 2
    });

    let mut index_1 = -1;
    let mut index_2 = -1;
    let _subscription = observable
        .flat_map(|v| {
            index_1 += 1;
            let i = index_1;
            v.map(move |v| (i, v))
        })
        .flat_map(|v| {
            index_2 += 1;
            let i0 = v.0;
            let i = index_2;
            v.1.map(move |v| 100 * i0 + 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 111, 22, 133, 4, 115, 26, 137, 8, 119]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [0, 111, 22, 133, 4, 115, 26, 137, 8, 119]);
    assert!(checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = GroupBy::new(observable, |value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_revert_completed() {
    let mut subject: PublishSubject<'_, i32, &'static str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable.clone().merge();
    let observable_2 = observable.clone().concat();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert!(checker_2.is_completed());
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert!(checker_3.is_completed());
}

#[test]
fn test_revert_error() {
    let mut subject: PublishSubject<'_, i32, &'static str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable.clone().merge();
    let observable_2 = observable.clone().concat();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert!(checker_2.is_error("error"));
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert!(checker_3.is_error("error"));
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

        let observable = observable.group_by(|value| value.to_string());

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
            Subscription::new_none_disposal()
        });
        let observable = observable.group_by(|v: i32| v).merge().map(|_| None);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker_2));
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| {
        s.consume_mut();
        value.to_string()
    });

    observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::new_none_disposal()
    });
    let observable = observable.group_by(|value| value);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.group_by(|value| value.to_string());

    let observable = observable.buffer_with_count(1);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.group_by(|value| value.to_string());

    observable.buffer_with_count(1);
}
