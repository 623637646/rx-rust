mod tests_utils;

use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, utility::do_on_termination::DoOnTermination},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use std::{
    convert::Infallible,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_termination(Termination::<&str>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let terminations = Arc::new(Mutex::new(Vec::new()));
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(|termination| {
        terminations.lock().unwrap().push(termination.clone());
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subscription_1.unsubscribe();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_error("error"));
    assert_eq!(
        terminations.lock().unwrap().deref(),
        &[Termination::Error("error")]
    );
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    let mut subject = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(|termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(&value);
    assert_eq!(checker_1.values(), [&value]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value]);
    assert!(checker_1.is_error(&error));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value);
        observer.on_termination(Termination::Error(&mut error));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::<(), _>::new();
    let (_, on_termination) = observer.into_callbacks();

    // Custom operations
    let observable = observable.do_on_termination(|termination| match termination {
        Termination::Completed => panic!(),
        Termination::Error(error) => {
            on_termination(Termination::Error(**error));
        }
    });

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                *error *= 2;
            }
        },
    );

    assert!(checker.values().is_empty());
    assert!(checker.is_error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(|termination| {
        observer_2.on_termination(termination.clone());
    });

    let handle = tokio::spawn(async move { observable.subscribe(observer_1) });
    let subscription = handle.await.unwrap();
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(111);
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_termination(Termination::Error("error"));
    });
    handle.await.unwrap();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_dropped());
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let terminations = Arc::new(Mutex::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let terminations_cloned = terminations.clone();
    let observable = observable.do_on_termination(move |termination| {
        terminations_cloned
            .lock()
            .unwrap()
            .push(termination.clone());
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
    assert_eq!(
        terminations.lock().unwrap().as_ref(),
        vec![Termination::Error("error"), Termination::Error("error")]
    );
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();
    let terminations = Arc::new(Mutex::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let terminations_cloned_1 = terminations.clone();
    let terminations_cloned_2 = terminations.clone();
    let observable = observable
        .do_on_termination(move |termination| {
            terminations_cloned_1
                .lock()
                .unwrap()
                .push(termination.clone());
        })
        .do_on_termination(move |termination| {
            terminations_cloned_2
                .lock()
                .unwrap()
                .push(termination.clone());
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(terminations.lock().unwrap().is_empty());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert_eq!(
        terminations.lock().unwrap().as_ref(),
        vec![Termination::Error("error"), Termination::Error("error")]
    );
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = DoOnTermination::new(observable, move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_error("error"));
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

        let observable = observable.do_on_termination(|_| {});

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
        let observable = observable.do_on_termination(|_| {});

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_on_termination(|_| {
        s.consume();
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
    let observable = observable.do_on_termination(|_| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_on_termination(|_| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_on_termination(|_| {});

    observable.filter(|_| true);
}
