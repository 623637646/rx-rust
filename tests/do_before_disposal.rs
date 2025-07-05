mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, utility::do_before_disposal::DoBeforeDisposal},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_cloned = called.clone();
    let observable = observable.do_before_disposal(|| {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_error() {
    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_cloned = called.clone();
    let observable = observable.do_before_disposal(|| {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_unsubscribe() {
    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_cloned = called.clone();
    let observable = observable.do_before_disposal(|| {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_cloned = called.clone();
    let observable = observable.do_before_disposal(|| {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_active());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_error(&error));
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer: BoxedObserver<'_, &mut i32, &mut i32>| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });

    // Custom operations
    let called_cloned = called.clone();
    let observable = observable.do_before_disposal(|| {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| *value *= 2,
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => *error *= 2,
        },
    );
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(&mut value);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::Error(&mut error));
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(async {
        let disposed = Arc::new(AtomicBool::new(false));
        let called = Arc::new(AtomicBool::new(false));
        let boxed_observer = Arc::new(Mutex::new(None));

        let disposed_cloned = disposed.clone();
        let boxed_observer_cloned = boxed_observer.clone();
        let observable = Create::new(move |observer| {
            *boxed_observer_cloned.lock().unwrap() = Some(observer);
            Subscription::new_with_disposal_callback(move || {
                disposed_cloned.store(true, Ordering::SeqCst);
            })
        });
        let (checker, observer) = Checker::new();

        // Custom operations
        let disposed_cloned = disposed.clone();
        let called_cloned = called.clone();
        let observable = observable.do_before_disposal(move || {
            assert!(!disposed_cloned.load(Ordering::SeqCst));
            called_cloned.store(true, Ordering::SeqCst);
        });

        let handle = spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert_eq!(checker.values(), []);
        assert!(checker.is_active());
        assert!(!disposed.load(Ordering::SeqCst));
        assert!(!called.load(Ordering::SeqCst));

        let handle = spawn(async move {
            boxed_observer
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .on_next(111);
            boxed_observer
        });
        let boxed_observer = handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(!disposed.load(Ordering::SeqCst));
        assert!(!called.load(Ordering::SeqCst));

        let handle = spawn(async move { subscription.dispose() });
        handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(disposed.load(Ordering::SeqCst));
        assert!(called.load(Ordering::SeqCst));

        let handle = spawn(async move {
            boxed_observer
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .on_termination(Termination::<Infallible>::Completed);
        });
        handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
        assert!(disposed.load(Ordering::SeqCst));
        assert!(called.load(Ordering::SeqCst));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let disposed_1 = Arc::new(AtomicBool::new(false));
    let disposed_2 = Arc::new(AtomicBool::new(false));
    let called_1 = Arc::new(AtomicBool::new(false));
    let called_2 = Arc::new(AtomicBool::new(false));
    let boxed_observer_1 = Arc::new(Mutex::new(None));
    let boxed_observer_2 = Arc::new(Mutex::new(None));

    let observable = Create::new(|observer| {
        if boxed_observer_1.lock().unwrap().is_none() {
            *boxed_observer_1.lock().unwrap() = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                disposed_1.store(true, Ordering::SeqCst);
            })
        } else {
            *boxed_observer_2.lock().unwrap() = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                disposed_2.store(true, Ordering::SeqCst);
            })
        }
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let called_1_cloned = called_1.clone();
    let called_2_cloned = called_2.clone();
    let first_call = Arc::new(AtomicBool::new(true));
    let observable = observable.do_before_disposal(|| {
        if first_call.load(Ordering::SeqCst) {
            first_call.store(false, Ordering::SeqCst);
            assert!(!disposed_1.load(Ordering::SeqCst));
            assert!(!disposed_2.load(Ordering::SeqCst));
            called_1_cloned.store(true, Ordering::SeqCst);
        } else {
            assert!(disposed_1.load(Ordering::SeqCst));
            assert!(!disposed_2.load(Ordering::SeqCst));
            called_2_cloned.store(true, Ordering::SeqCst);
        }
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), []);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), []);
    assert!(checker_2.is_active());
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    boxed_observer_1
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .on_next(111);
    boxed_observer_2
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    subscription_1.dispose();
    subscription_2.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));

    boxed_observer_1
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    boxed_observer_2
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_completed());
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));
}

#[test]
fn test_multiple_operation() {
    let disposed = Arc::new(AtomicBool::new(false));
    let called_1 = Arc::new(AtomicBool::new(false));
    let called_2 = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_1_cloned = called_1.clone();
    let called_2_cloned = called_2.clone();
    let observable = observable
        .do_before_disposal(|| {
            assert!(!disposed.load(Ordering::SeqCst));
            called_1_cloned.store(true, Ordering::SeqCst);
        })
        .do_before_disposal(|| {
            assert!(!disposed.load(Ordering::SeqCst));
            called_2_cloned.store(true, Ordering::SeqCst);
        });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));
}

#[test]
fn test_without_convenient_api() {
    let disposed = Arc::new(AtomicBool::new(false));
    let called = Arc::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new_with_disposal_callback(|| {
            disposed.store(true, Ordering::SeqCst);
        })
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let called_cloned = called.clone();
    let observable = DoBeforeDisposal::new(observable, || {
        assert!(!disposed.load(Ordering::SeqCst));
        called_cloned.store(true, Ordering::SeqCst);
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
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

        let observable = observable.do_before_disposal(|| {});

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
        let observable = observable.do_before_disposal(|| {});

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
    let observable = observable.do_before_disposal(|| {
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
    let observable = observable.do_before_disposal(|| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_before_disposal(|| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_before_disposal(|| {});

    observable.filter(|_| true);
}
