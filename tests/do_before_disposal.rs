mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::safe_lock_option;
use rx_rust::safe_lock_option_observer;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, utility::do_before_disposal::DoBeforeDisposal},
    subject::publish_subject::PublishSubject,
};
use std::{
    convert::Infallible,
    sync::atomic::{AtomicBool, Ordering},
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_error() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_unsubscribe() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer: BoxedObserver<'_, &mut i32, &mut i32>| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    block_on(|runtime| async move {
        let disposed = Shared::new(AtomicBool::new(false));
        let called = Shared::new(AtomicBool::new(false));
        let boxed_observer = Shared::new(Mutable::new(None));

        let disposed_cloned = disposed.clone();
        let boxed_observer_cloned = boxed_observer.clone();
        let observable = Create::new(move |observer| {
            safe_lock_option!(replace: boxed_observer_cloned, observer);
            Subscription::new(CallbackDisposal::new(move || {
                disposed_cloned.store(true, Ordering::SeqCst);
            }))
        });
        let (checker, observer) = Checker::new();

        // Custom operations
        let disposed_cloned = disposed.clone();
        let called_cloned = called.clone();
        let observable = observable.do_before_disposal(move || {
            assert!(!disposed_cloned.load(Ordering::SeqCst));
            called_cloned.store(true, Ordering::SeqCst);
        });

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert!(!disposed.load(Ordering::SeqCst));
        assert!(!called.load(Ordering::SeqCst));

        let boxed_observer = runtime
            .spawn(async move {
                assert!(safe_lock_option_observer!(on_next: boxed_observer, 111));
                boxed_observer
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert!(!disposed.load(Ordering::SeqCst));
        assert!(!called.load(Ordering::SeqCst));

        runtime
            .spawn(async move { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert!(disposed.load(Ordering::SeqCst));
        assert!(called.load(Ordering::SeqCst));

        runtime
            .spawn(async move {
                assert!(
                    safe_lock_option_observer!(on_termination: boxed_observer, Termination::<Infallible>::Completed)
                );
            })
            .await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert!(disposed.load(Ordering::SeqCst));
        assert!(called.load(Ordering::SeqCst));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let called_1 = Shared::new(AtomicBool::new(false));
    let called_2 = Shared::new(AtomicBool::new(false));
    let boxed_observer_1 = Shared::new(Mutable::new(None));
    let boxed_observer_2 = Shared::new(Mutable::new(None));

    let observable = Create::new(|observer| {
        let disposed = if safe_lock_option!(is_none: boxed_observer_1) {
            safe_lock_option!(replace: boxed_observer_1, observer);
            disposed_1.clone()
        } else {
            safe_lock_option!(replace: boxed_observer_2, observer);
            disposed_2.clone()
        };
        Subscription::new(CallbackDisposal::new(move || {
            disposed.store(true, Ordering::SeqCst);
        }))
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let called_1_cloned = called_1.clone();
    let called_2_cloned = called_2.clone();
    let first_call = Shared::new(AtomicBool::new(true));
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
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    assert!(safe_lock_option_observer!(on_next: boxed_observer_1, 111));
    assert!(safe_lock_option_observer!(on_next: boxed_observer_2, 111));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    subscription_1.dispose();
    subscription_2.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));

    assert!(
        safe_lock_option_observer!(on_termination: boxed_observer_1, Termination::<Infallible>::Completed)
    );
    assert!(
        safe_lock_option_observer!(on_termination: boxed_observer_2, Termination::<Infallible>::Completed)
    );
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));
}

#[test]
fn test_unsub_on_next_by_take() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let called_cloned = called.clone();
    let mut boxed_observer = None;

    let observable = Create::new(|observer: BoxedObserver<'_, _, Infallible>| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
    });
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .do_before_disposal(|| {
            assert!(!disposed.load(Ordering::SeqCst));
            called_cloned.store(true, Ordering::SeqCst);
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_multiple_operation() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called_1 = Shared::new(AtomicBool::new(false));
    let called_2 = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called_1.load(Ordering::SeqCst));
    assert!(!called_2.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called_1.load(Ordering::SeqCst));
    assert!(called_2.load(Ordering::SeqCst));
}

#[test]
fn test_without_convenient_api() {
    let disposed = Shared::new(AtomicBool::new(false));
    let called = Shared::new(AtomicBool::new(false));
    let mut boxed_observer = None;

    let observable = Create::new(|observer| {
        boxed_observer = Some(observer);
        Subscription::new(CallbackDisposal::new(|| {
            disposed.store(true, Ordering::SeqCst);
        }))
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
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    boxed_observer.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!disposed.load(Ordering::SeqCst));
    assert!(!called.load(Ordering::SeqCst));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(disposed.load(Ordering::SeqCst));
    assert!(called.load(Ordering::SeqCst));

    boxed_observer
        .unwrap()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
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
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
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
            Subscription::default()
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

    let _ = observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
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
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_before_disposal(|| {});

    observable.filter(|_| true);
}
