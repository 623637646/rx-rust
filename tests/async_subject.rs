mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::DropProbe;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::observable::Observable;
use rx_rust::observable::ObservableExt;
use rx_rust::observer::{Flow, Observer, Termination};
use rx_rust::subject::Subject;
use rx_rust::subject::async_subject::AsyncSubject;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // The subject is terminated before its observers are told.
            assert!(subject_cloned.termination().is_some());
        },
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));

    // on_next and on_termination after termination
    assert!(subject.on_next(333).is_stop());
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_error() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // The subject is terminated before its observers are told.
            assert!(subject_cloned.termination().is_some());
        },
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.termination(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    assert!(subject.on_next(333).is_stop());
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.termination(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.termination(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsubscribe() {
    let mut subject = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(&value_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(&value_2).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [&value_2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let subject = AsyncSubject::shared();
        let (checker, observer) = Checker::new();

        let observable = subject.clone();

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert!(subject.termination().is_none());

        let mut subject_cloned = subject.clone();
        scheduler
            .spawn(async move {
                assert!(subject_cloned.on_next(&111).is_continue());
            })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert!(subject.termination().is_none());

        let subject_cloned = subject.clone();
        scheduler
            .spawn(async move {
                subject_cloned.on_termination(Termination::<Infallible>::Completed);
            })
            .await;
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Completed);
        assert!(matches!(
            subject.termination(),
            Some(Termination::Completed)
        ));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_unsub_on_next_by_take() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone().take(1);

    let _subscription = observable.clone().subscribe(observer);
    let subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        |_| {},
        move |_| {
            // The completion is queued together with the last value, so the subject is already
            // terminated when `take` passes that value on and terminates this observer.
            assert!(subject_cloned.termination().is_some());
        },
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_complete_on_next() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            if value < 3 {
                assert!(subject_cloned.on_next(value + 1).is_stop());
                subject_cloned
                    .clone()
                    .on_termination(Termination::Completed);
            }
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(1).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    // The subject is terminated by the very step that queued `1` and the completion, so the `2`
    // the callback of `1` sends arrives once it has terminated and is dropped.
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_error_on_next() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

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
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    // The completion is queued together with `111` and terminates the subject at once, so the
    // error the callback of `111` sends is dropped.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_unsub_on_next() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                observer.on_next(value)
            })
            .subscribe(observer_2),
    ));

    // unsubscribe after on_next
    let sub = Arc::new(Mutex::new(None));
    let sub_cloned = sub.clone();
    sub.replace_value(Some(
        observable
            .hook_on_next(move |observer, value| {
                assert!(observer.on_next(value).is_continue());
                if let Some(subscription) = sub_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                Flow::Continue
            })
            .subscribe(observer_3),
    ));

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_sub_on_next() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

    let subscription_2_cloned = subscription_2.clone();
    let subscription_3_cloned = subscription_3.clone();
    let _subscription = observable
        .clone()
        .hook_on_next(move |observer, value| {
            // subscribe before on_next
            if let Some(observer) = observer_2.take() {
                subscription_2_cloned.replace_value(Some(observable.clone().subscribe(observer)));
            }
            assert!(observer.on_next(value).is_continue());
            // subscribe after on_next
            if let Some(observer) = observer_3.take() {
                subscription_3_cloned.replace_value(Some(observable.clone().subscribe(observer)));
            }
            Flow::Continue
        })
        .subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    // Both subscribed while `222` and the completion were queued together, so both arrived at an
    // already terminated subject and were replayed its last value before being completed.
    assert_eq!(checker_1.values(), [222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [222]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_next_on_next() {
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = subject.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            // The last value and the completion are queued in one step, so the subject is already
            // terminated when that value is delivered.
            assert!(subject_cloned.termination().is_some());
            if value < 3 {
                assert!(subject_cloned.on_next(value + 1).is_stop());
                subject_cloned
                    .clone()
                    .on_termination(Termination::Completed);
            }
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(1).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.termination().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    // The subject is terminated by the very step that queued `1` and the completion, so the `2`
    // the callback of `1` sends arrives once it has terminated and is dropped.
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_unsub_on_completed() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Arc::new(Mutex::new(None));
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
    let sub = Arc::new(Mutex::new(None));
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

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_sub_on_completed() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

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
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [222]);
    assert_eq!(checker_3.state(), State::Completed);
    assert!(matches!(
        subject.termination(),
        Some(Termination::Completed)
    ));
}

#[test]
fn test_unsub_on_error() {
    let mut subject = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    // no unsubscribe
    let _subscription = Some(observable.clone().subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Arc::new(Mutex::new(None));
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
    let sub = Arc::new(Mutex::new(None));
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

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.termination(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_sub_on_error() {
    let mut subject = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let mut observer_3 = Some(observer_3);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_3 = Arc::new(Mutex::new(None));

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
    assert!(subject.termination().is_none());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.termination().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert!(matches!(
        subject.termination(),
        Some(Termination::Error("error"))
    ));
}

// The cases below re-enter the subject, or race it from other threads, where an `AsyncSubject`
// that kept its last value under a lock of its own would let another event slip in between two
// steps. The value lives in the multicast's resources instead, so reading it, recording the
// termination and queueing the events are one step under one lock.

#[test]
fn test_next_on_replayed_next() {
    // A value sent from the callback of the replayed value must not replace what later subscribers
    // are replayed: the termination is recorded by the step that queues the value.
    let mut subject = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let terminated_while_replaying = Arc::new(Mutex::new(Vec::new()));

    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer_1);
    let mut subject_cloned = subject.clone();
    let terminated_cloned = terminated_while_replaying.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        move |value| {
            terminated_cloned
                .with_mut(|values| values.push(subject_cloned.termination().is_some()));
            if value == 111 {
                assert!(subject_cloned.on_next(222).is_stop());
            }
        },
        move |_| {},
    );

    assert!(subject.on_next(111).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);

    // The value is replayed by `on_termination`, so the subject is already terminated while the
    // callback runs and the `222` it sends is dropped.
    assert_eq!(terminated_while_replaying.clone_value(), [true]);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);

    // A subscriber arriving after the termination is replayed what the earlier ones saw.
    let (checker_2, observer_2) = Checker::new();
    let _subscription = observable.subscribe(observer_2);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_complete_on_replayed_next() {
    // A termination sent from the callback of the replayed value must not deliver the value a
    // second time: an `AsyncSubject` emits its value once.
    let mut subject = AsyncSubject::shared();
    let (checker, observer) = Checker::new();

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

    assert!(subject.on_next(111).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_sub_on_replayed_next() {
    // The value is replayed by `on_termination`, so an observer subscribing from that callback
    // arrives at a terminated subject, and is replayed the value too.
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();

    let mut observer_2 = Some(observer_2);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_2_cloned = subscription_2.clone();
    let observable_cloned = observable.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        move |_| {
            if let Some(observer) = observer_2.take() {
                subscription_2_cloned
                    .replace_value(Some(observable_cloned.clone().subscribe(observer)));
            }
        },
        move |_| {},
    );
    let _subscription = observable.subscribe(observer_1);

    assert!(subject.on_next(111).is_continue());
    subject.clone().on_termination(Termination::Completed);

    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_next_on_sub() {
    // Replaying the value to a late subscriber runs its callback: the subject has terminated long
    // ago, so a value sent from there is dropped and the replayed value stays what it was.
    let mut subject = AsyncSubject::shared();
    assert!(subject.on_next(111).is_continue());
    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);

    let observable = subject.clone();

    let mut subject_cloned = subject.clone();
    let _subscription = observable.clone().subscribe_with_callback(
        move |value| {
            if value == 111 {
                assert!(subject_cloned.on_next(222).is_stop());
            }
        },
        move |_| {},
    );

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_next_on_unsub() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    assert!(subject.on_next(111).is_continue());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();

    // The second observer sends a value into the subject while it is being released, so the value
    // is sent from inside the unsubscription. It becomes the value the subject replays.
    let mut subject_cloned = subject.clone();
    let probe = DropProbe::new().on_drop(Box::new(move || {
        assert!(subject_cloned.on_next(222).is_continue());
    }));
    let (mut next_2, termination_2) = observer_2.into_callbacks();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subscription_2 = observable.subscribe_with_callback(
        move |value| {
            let _ = &probe;
            next_2(value);
        },
        termination_2,
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subscription_2.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(subject.termination().is_none());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [222]);
    assert_eq!(checker_1.state(), State::Completed);
}

#[test]
fn test_complete_on_unsub() {
    let subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    let mut subject_seed = subject.clone();
    assert!(subject_seed.on_next(111).is_continue());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();

    // The second observer completes the subject while it is being released, so the subject
    // terminates from inside the unsubscription.
    let subject_cloned = subject.clone();
    let probe = DropProbe::new().on_drop(Box::new(move || {
        subject_cloned.on_termination(Termination::Completed);
    }));
    let (mut next_2, termination_2) = observer_2.into_callbacks();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subscription_2 = observable.subscribe_with_callback(
        move |value| {
            let _ = &probe;
            next_2(value);
        },
        termination_2,
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subscription_2.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(subject.termination().is_some());
}

#[test]
fn test_error_on_unsub() {
    let subject: AsyncSubject<'_, _, &str, _> = AsyncSubject::shared();
    let mut subject_seed = subject.clone();
    assert!(subject_seed.on_next(111).is_continue());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();

    // The second observer fails the subject while it is being released, so the subject terminates
    // from inside the unsubscription.
    let subject_cloned = subject.clone();
    let probe = DropProbe::new().on_drop(Box::new(move || {
        subject_cloned.on_termination(Termination::Error("error"));
    }));
    let (mut next_2, termination_2) = observer_2.into_callbacks();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subscription_2 = observable.subscribe_with_callback(
        move |value| {
            let _ = &probe;
            next_2(value);
        },
        termination_2,
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subscription_2.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(subject.termination().is_some());
}

#[test]
fn test_sub_on_sub() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    assert!(subject.on_next(111).is_continue());
    subject.clone().on_termination(Termination::Completed);
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = subject.clone();

    // The first observer subscribes the second one from the delivery of the value it is
    // replayed, which runs while its own subscription is still in progress.
    let observable_cloned = observable.clone();
    let mut observer_2 = Some(observer_2);
    let subscription_2 = Arc::new(Mutex::new(None));
    let subscription_2_cloned = subscription_2.clone();
    let (mut next_1, termination_1) = observer_1.into_callbacks();

    let _subscription_1 = observable.subscribe_with_callback(
        move |value| {
            if let Some(observer_2) = observer_2.take() {
                subscription_2_cloned
                    .replace_value(Some(observable_cloned.clone().subscribe(observer_2)));
            }
            next_1(value);
        },
        termination_1,
    );
    assert!(subscription_2.with_ref(Option::is_some));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_sub_on_unsub() {
    let mut subject: AsyncSubject<'_, _, Infallible, _> = AsyncSubject::shared();
    assert!(subject.on_next(111).is_continue());
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    let observable = subject.clone();

    // The second observer subscribes the third one while it is being released, so that
    // subscription runs from inside the unsubscription.
    let observable_cloned = observable.clone();
    let subscription_3 = Arc::new(Mutex::new(None));
    let subscription_3_cloned = subscription_3.clone();
    let probe = DropProbe::new().on_drop(Box::new(move || {
        subscription_3_cloned.replace_value(Some(observable_cloned.subscribe(observer_3)));
    }));
    let (mut next_2, termination_2) = observer_2.into_callbacks();

    let _subscription_1 = observable.clone().subscribe(observer_1);
    let subscription_2 = observable.subscribe_with_callback(
        move |value| {
            let _ = &probe;
            next_2(value);
        },
        termination_2,
    );
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    subscription_2.dispose();
    assert!(subscription_3.with_ref(Option::is_some));
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Completed);
}

#[test]
fn test_race_condition() {
    // The subscribers race the termination: reading the termination and the value is one step, so
    // an observer is never terminated without the value, whichever side of the termination it
    // lands on.
    use std::sync::{Arc, Barrier};

    const SUBSCRIBERS: usize = 3;
    const ROUNDS: usize = 20;

    for _ in 0..200 {
        let mut subject: AsyncSubject<'static, i32, Infallible, _> = AsyncSubject::shared();
        assert!(subject.on_next(111).is_continue());
        let barrier = Arc::new(Barrier::new(SUBSCRIBERS + 1));

        let handles: Vec<_> = (0..SUBSCRIBERS)
            .map(|_| {
                let subject = subject.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    // The subscriptions are returned so that the observers stay subscribed until
                    // every thread has finished: an observer that joined before the termination is
                    // notified by it, not by `subscribe`.
                    (0..ROUNDS)
                        .map(|_| {
                            let (checker, observer) = Checker::new();
                            (checker, subject.clone().subscribe(observer))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        barrier.wait();
        subject.clone().on_termination(Termination::Completed);

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        for (checker, _subscription) in results.iter().flatten() {
            // Whether the observer joined the subject before the termination or was replayed by
            // it, it sees the last value followed by the completion.
            assert_eq!(checker.values(), [111]);
            assert_eq!(checker.state(), State::Completed);
        }
    }
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
        assert!(observer.on_next(Some(&life_marker)).is_continue());
        let subject = AsyncSubject::shared();
        _subscription = subject.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = AsyncSubject::<TestStruct, TestStruct, _>::shared();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let subject: AsyncSubject<'_, i32, String, _> = AsyncSubject::shared();
    let observable = subject;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let subject: AsyncSubject<'_, i32, String, _> = AsyncSubject::shared();
    let observable = subject;

    observable.filter(|_| true);
}
