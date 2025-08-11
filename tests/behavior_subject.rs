mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::safe_lock_option;
use rx_rust::safe_lock_option_disposable;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::Subject;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, Shared};
use std::convert::Infallible;
use std::time::Duration;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 111);

    // on_next and on_termination after termination
    subject.on_next(222);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 111);

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 111);
}

#[test]
fn test_error() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // Terminate Subject itself first. Then terminate Observers in Subject.
            assert!(subject_cloned.terminated().is_some());
        },
    );
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    // on_next and on_termination after termination
    subject.on_next(222);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [-1, 111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 111);

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 111);
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
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [-1, 111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 222);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [-1, 111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 222);
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

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &value_1);

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), &value_2);

    subject.clone().on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error(&222))
    ));
    assert_eq!(subject.value(), &value_2);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = BehaviorSubject::new(&-1);
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&-1]);
        assert_eq!(checker.state(), State::Active);
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &-1);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(&111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&-1, &111]);
        assert_eq!(checker.state(), State::Active);
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &111);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [&-1, &111]);
        assert_eq!(checker.state(), State::Dropped);
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &111);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&-1, &111]);
        assert_eq!(checker.state(), State::Dropped);
        assert!(matches!(
            subject.terminated(),
            Some(Termination::Error("error"))
        ));
        assert_eq!(subject.value(), &111);
    });
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

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 111);
}

#[test]
fn test_unsub_on_next_by_take() {
    let subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().take(1);

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // In this case, Subject is not terminated.
            assert!(subject_cloned.terminated().is_none());
        },
    );
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::<Infallible>::Completed);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);
}

#[test]
fn test_complete_on_next() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), -1);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), -1);
}

#[test]
fn test_error_on_next() {
    let mut subject = BehaviorSubject::new(-1);
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
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), -1);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [-1]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), -1);
}

#[test]
fn test_unsub_on_next() {
    let mut subject: BehaviorSubject<'_, _, Infallible> = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
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
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);
}

#[test]
fn test_sub_on_next() {
    let mut subject: BehaviorSubject<'_, _, Infallible> = BehaviorSubject::new(-1);
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
    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [-1, 111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1, 111, 222]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
}

#[test]
fn test_next_on_next() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            assert!(subject_cloned.terminated().is_none());
            if value < 3 {
                subject_cloned.on_next(value + 1);
            }
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert_eq!(checker.values(), [-1, 0]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 0);

    subject.on_next(1);
    assert_eq!(checker.values(), [-1, 0]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 0);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [-1, 0]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 0);
}

#[test]
fn test_unsub_on_completed() {
    let mut subject: BehaviorSubject<'_, _, Infallible> = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
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
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    assert_eq!(subject.value(), 111);
}

#[test]
fn test_sub_on_completed() {
    let mut subject: BehaviorSubject<'_, _, Infallible> = BehaviorSubject::new(-1);
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
    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_error() {
    let mut subject = BehaviorSubject::new(-1);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
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
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), -1);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
    assert_eq!(subject.value(), 111);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [-1, 111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [-1, 111]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
    assert_eq!(subject.value(), 111);
}

#[test]
fn test_sub_on_error() {
    let mut subject = BehaviorSubject::new(-1);
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
    assert_eq!(checker_1.values(), [-1]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [-1, 111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Error("error"));
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
        let subject = BehaviorSubject::new(None);
        _subscription = subject.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = BehaviorSubject::<'_, _, TestStruct>::new(TestStruct);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: BehaviorSubject<'_, i32, String> = BehaviorSubject::new(-1);
    let observable = subject;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: BehaviorSubject<'_, i32, String> = BehaviorSubject::new(-1);
    let observable = subject;

    observable.filter(|_| true);
}
