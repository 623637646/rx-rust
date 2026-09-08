mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::observable::Observable;
use rx_rust::observable::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::Subject;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::utils::mutable::Mutable;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::utils::types::Shared;
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
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
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(222);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
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
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(222);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error(&222))
    ));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert!(subject.terminated().is_none());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(&111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Active);
        assert!(subject.terminated().is_none());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
        assert!(subject.terminated().is_none());

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Dropped);
        assert!(matches!(
            subject.terminated(),
            Some(Termination::Error("error"))
        ));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsub_on_next_by_take() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
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
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(subject.terminated().is_none());
}

#[test]
fn test_complete_on_next() {
    let mut subject = PublishSubject::default();
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
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error_on_next() {
    let mut subject = PublishSubject::default();
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
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsub_on_next() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_next(value);
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(subject.terminated().is_none());
}

// An observer that another observer disposes mid-dispatch must not receive the value that is
/// being dispatched, whether it is disposed before or after it would have been visited.
#[test]
fn test_unsub_other_on_next() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // Subscribed first, so it is visited first and disposes the other two before they are.
    let subscription_1 = Shared::new(Mutable::new(None));
    let subscription_3 = Shared::new(Mutable::new(None));
    let subscription_1_cloned = subscription_1.clone();
    let subscription_3_cloned = subscription_3.clone();
    subscription_1.replace_value(Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                // Disposes the observer that was already visited, and the one that was not.
                if let Some(subscription) = subscription_1_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                if let Some(subscription) = subscription_3_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_next(value);
            })
            .subscribe(observer_1),
    ));
    let _subscription_2 = observable.clone().subscribe(observer_2);
    subscription_3.replace_value(Some(observable.subscribe(observer_3)));

    subject.on_next(111);
    // The observer that disposed itself still gets the value it is being delivered.
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    // Disposed before it was visited, so the in-flight value must not reach it.
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_3.values(), []);
}

#[test]
fn test_sub_on_next() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
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
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                observer.on_next(value);
                // subscribe after on_next
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
            })
            .subscribe(observer_1),
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [222]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());
}

#[test]
fn test_next_on_next() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            // The subject is terminated as soon as `on_termination` is called, which the callback
            // of `1` did: the callback of `2` sees it terminated while the termination is still
            // queued behind that value.
            assert_eq!(subject_cloned.terminated().is_some(), value > 1);
            if value < 3 {
                subject_cloned.on_next(value + 1);
            }
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
        },
        move |_| {},
    );
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(1);
    // The events are delivered in the order they were sent: `3` is sent from the callback of `2`,
    // after that callback has already sent the termination, so it arrives once the subject has
    // terminated and is dropped.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_completed() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_sub_on_completed() {
    let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();
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
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
            })
            .subscribe(observer_1),
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_error() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
            })
            .subscribe(observer_3),
    ));

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_sub_on_error() {
    let mut subject = PublishSubject::default();
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
                    subscription_2_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
                observer.on_termination(value);
                // subscribe after termination
                if let Some(observer) = observer_3.take() {
                    subscription_3_cloned
                        .replace_value(Some(observable.clone().subscribe(observer)));
                }
            })
            .subscribe(observer_1),
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
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
        observer.on_next(&life_marker);
        let subject = PublishSubject::default();
        _subscription = subject.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = PublishSubject::<'_, TestStruct, TestStruct>::default();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject;

    observable.filter(|_| true);
}

/// Every observer is notified in subscription order, and unsubscribing one does not disturb the
/// order of the others.
#[test]
fn test_notification_order() {
    /// What one observer received, tagged with the index of the observer that received it.
    type Notification = (usize, Option<i32>);

    let mut subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let notifications: Shared<Mutable<Vec<Notification>>> = Shared::new(Mutable::new(Vec::new()));

    let observable = subject.clone();
    let subscribe = |index: usize| {
        let notifications_next = notifications.clone();
        let notifications_termination = notifications.clone();
        observable.clone().subscribe_with_callback(
            move |value| notifications_next.with_mut(|values| values.push((index, Some(value)))),
            move |_| notifications_termination.with_mut(|values| values.push((index, None))),
        )
    };
    let mut subscriptions: Vec<_> = (0..5).map(&subscribe).collect();

    // Custom operations
    subject.on_next(111);
    assert_eq!(
        notifications.take_value(),
        [
            (0, Some(111)),
            (1, Some(111)),
            (2, Some(111)),
            (3, Some(111)),
            (4, Some(111)),
        ]
    );

    // The remaining observers keep their relative order after two of them unsubscribed.
    subscriptions.remove(3).dispose();
    subscriptions.remove(1).dispose();
    subject.on_next(222);
    assert_eq!(
        notifications.take_value(),
        [(0, Some(222)), (2, Some(222)), (4, Some(222))]
    );

    // A later subscriber is notified last.
    subscriptions.push(subscribe(5));
    subject.on_next(333);
    assert_eq!(
        notifications.take_value(),
        [
            (0, Some(333)),
            (2, Some(333)),
            (4, Some(333)),
            (5, Some(333)),
        ]
    );

    // The termination follows the same order.
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(
        notifications.take_value(),
        [(0, None), (2, None), (4, None), (5, None)]
    );

    drop(subscriptions);
}

// MARK: - Concurrency

/// An id is handed out and the entry carrying it queued under one lock, so entries reach the
/// delegate in the order their ids were handed out however many threads subscribe at once. Out of
/// order, the delegate's entries would no longer be sorted: the ids could not be found anymore, so
/// unsubscribing would stop releasing the observers.
#[cfg(not(feature = "single-threaded"))]
#[test]
fn observers_that_subscribe_concurrently_are_notified_and_released() {
    use crate::tests_utils::drop_probe::DropCount;

    const THREADS: usize = 8;
    const SUBSCRIPTIONS_PER_THREAD: usize = 25;
    const SUBSCRIPTIONS: usize = THREADS * SUBSCRIPTIONS_PER_THREAD;

    let mut subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let notifications: Shared<Mutable<Vec<i32>>> = Shared::new(Mutable::new(Vec::new()));
    let releases = DropCount::new();

    let subscriptions: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..THREADS)
            .map(|_| {
                let observable = subject.clone();
                let notifications = notifications.clone();
                let releases = releases.clone();
                scope.spawn(move || {
                    (0..SUBSCRIPTIONS_PER_THREAD)
                        .map(|_| {
                            let notifications = notifications.clone();
                            // Released with the observer, once its entry is pruned.
                            let probe = releases.probe();
                            observable.clone().subscribe_with_callback(
                                move |value| {
                                    let _ = &probe;
                                    notifications.with_mut(|values| values.push(value))
                                },
                                |_| {},
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().unwrap())
            .collect()
    });

    // Every observer that subscribed is subscribed, whichever thread subscribed it.
    subject.on_next(111);
    assert_eq!(notifications.take_value(), [111; SUBSCRIPTIONS]);
    assert_eq!(releases.get(), 0);

    // Every entry is found and pruned, which only holds while the entries are sorted by their id.
    drop(subscriptions);
    assert_eq!(releases.get(), SUBSCRIPTIONS);
    subject.on_next(222);
    assert!(notifications.with_ref(Vec::is_empty));
}

/// The subject is terminated as soon as `on_termination` is called, so an observer that subscribes
/// while the termination is still queued is terminated at once — before the observers that were
/// already subscribed, which the queued termination reaches later.
#[test]
fn a_subscriber_that_arrives_while_the_termination_is_queued_is_terminated_at_once() {
    let mut subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let notifications: Shared<Mutable<Vec<&str>>> = Shared::new(Mutable::new(Vec::new()));

    let observable = subject.clone();
    let notifications_first = notifications.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        move |_| notifications_first.with_mut(|values| values.push("first: value")),
        {
            let notifications = notifications.clone();
            move |_| notifications.with_mut(|values| values.push("first: termination"))
        },
    );

    let subject_cloned = subject.clone();
    let observable_cloned = observable.clone();
    let notifications_second = notifications.clone();
    let notifications_late = notifications.clone();
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            // The termination is queued behind the value being dispatched right now.
            subject_cloned
                .clone()
                .on_termination(Termination::Completed);
            assert!(subject_cloned.terminated().is_some());
            let notifications = notifications_late.clone();
            let _subscription = observable_cloned.clone().subscribe_with_callback(
                |_| unreachable!("the subject has terminated"),
                move |_| notifications.with_mut(|values| values.push("late: termination")),
            );
        },
        {
            let notifications = notifications_second.clone();
            move |_| notifications.with_mut(|values| values.push("second: termination"))
        },
    );

    subject.on_next(111);
    assert_eq!(
        notifications.take_value(),
        [
            "first: value",
            "late: termination",
            "first: termination",
            "second: termination",
        ]
    );
}
