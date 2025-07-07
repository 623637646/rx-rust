mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, sleep, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, others::hook_on_termination::HookOnTermination},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{
    convert::Infallible,
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.hook_on_termination(move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Error("error"));
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

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_completed());
}

#[test]
fn test_completed_no_call_original() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.hook_on_termination(move |_, termination| {
        observer_2.on_termination(termination);
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

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_dropped());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_completed());
}

#[test]
fn test_completed_send_values_before_terminated() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.hook_on_termination(move |mut observer, termination| {
        observer.on_next(222);
        observer.on_termination(termination);
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.hook_on_termination(move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Completed);
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
    assert!(checker_1.is_completed());
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
    let observable = observable.hook_on_termination(|observer, termination| {
        match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                assert_eq!(error, "error");
                observer.on_termination(Termination::Error("hooked"));
            }
        }
        terminations.lock().unwrap().push(termination);
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

    subscription_1.dispose();
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
    assert!(checker_2.is_error("hooked"));
    assert_eq!(
        terminations.lock().unwrap().deref(),
        &[Termination::Error("error")]
    );
}

#[test]
fn test_ref() {
    let value = 111;
    let error_1 = 222;
    let error_2 = 333;

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    let mut subject = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.hook_on_termination(|observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Error(&error_2));
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

    subject.on_termination(Termination::Error(&error_1));
    assert_eq!(checker_1.values(), [&value]);
    assert!(checker_1.is_error(&error_2));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_error(&error_1));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error_1 = 222;
    let mut error_2 = 333;

    let observable = Create::new(|mut observer| {
        observer.on_next(&mut value);
        observer.on_termination(Termination::Error(&mut error_1));
        Subscription::new_none_disposal()
    });
    let (checker, observer) = Checker::<Infallible, _>::new();
    let (_, on_termination) = observer.into_callbacks();

    // Custom operations
    let observable = observable.hook_on_termination(|observer, mut termination| {
        match &mut termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(**error));
                **error *= 2;
            }
        }
        observer.on_termination(Termination::Error(&mut error_2));
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
    assert_eq!(error_1, 444);
    assert_eq!(error_2, 666);
}

#[test]
fn test_async() {
    block_on(async {
        let subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.hook_on_termination(move |observer, termination| {
            observer_2.on_termination(termination);
            observer.on_termination(Termination::Completed);
            panic!()
        });

        let subscription = spawn(async move { observable.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        let mut subject_cloned = subject.clone();
        spawn(async move {
            subject_cloned.on_next(111);
        })
        .await
        .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        spawn(async { subscription.dispose() }).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_dropped());

        let subject_cloned = subject.clone();
        spawn(async move {
            subject_cloned.on_termination(Termination::Error("error"));
        })
        .await
        .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_dropped());
    });
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
    let observable = observable.hook_on_termination(move |observer, termination| {
        terminations_cloned.lock().unwrap().push(termination);
        observer.on_termination(Termination::Completed);
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
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
    assert_eq!(
        terminations.lock().unwrap().as_ref(),
        vec![Termination::Error("error"), Termination::Error("error")]
    );
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();
    let (checker_3, observer_3) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .hook_on_termination(move |observer, termination| {
            observer_2.on_termination(termination);
            observer.on_termination(Termination::Error("222"));
        })
        .hook_on_termination(move |observer, termination| {
            observer_3.on_termination(termination);
            observer.on_termination(Termination::Error("333"));
        });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Error("111"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("333"));
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_error("111"));
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_error("222"));
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = HookOnTermination::new(observable, move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Completed);
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
    assert!(checker_1.is_completed());
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

        let observable = observable.hook_on_termination(|_, _| {});

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
        let observable = observable.hook_on_termination(|_, _| {});

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
    let observable = observable.hook_on_termination(|_, _| {
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
    let observable = observable.hook_on_termination(|_, _| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.hook_on_termination(|_, _| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.hook_on_termination(|_, _| {});

    observable.filter(|_| true);
}
