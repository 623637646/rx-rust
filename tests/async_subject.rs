mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::subject::Subject;
use rx_rust::subject::async_subject::AsyncSubject;
use rx_rust::utils::types::{Mutable, MutableHelper, Shared};
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let mut subject = AsyncSubject::default();
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
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_error() {
    let mut subject = AsyncSubject::default();
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
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // on_next and on_termination after termination
    subject.on_next(333);
    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));

    // subscribe after termination
    let (checker, observer) = Checker::new();
    let _subscription = subject.clone().subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(matches!(
        subject.terminated(),
        Some(Termination::Error("error"))
    ));
}

#[test]
fn test_unsubscribe() {
    let mut subject = AsyncSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subscription_1.dispose();
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let mut subject = AsyncSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(&value_1);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(&value_2);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [&value_2]);
    assert_eq!(checker.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = AsyncSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let _subscription = runtime
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
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert!(subject.terminated().is_none());

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [&111]);
        assert_eq!(checker.state(), State::Completed);
        assert!(matches!(subject.terminated(), Some(Termination::Completed)));
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = AsyncSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
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
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(222);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

// AsyncSubject does not support this test. panic "No support for regression calls on_next"
// #[test]
// fn test_complete_on_next() {
//     let mut subject = AsyncSubject::default();
//     let (checker, observer) = Checker::new();

//     // Custom operations
//     let observable = subject.clone();

//     let _subscription = observable.clone().subscribe(observer);
//     let mut subject_cloned = Some(subject.clone());
//     let _subscription = observable.subscribe_with_callback(
//         move |_| {
//             subject_cloned
//                 .take()
//                 .unwrap()
//                 .on_termination(Termination::Completed);
//         },
//         move |_| {},
//     );
//     assert_eq!(checker.values(), []);
//     assert_eq!(checker.state(), State::Active);
//     assert!(subject.terminated().is_none());

//     subject.on_next(111);
//     assert_eq!(checker.values(), []);
//     assert_eq!(checker.state(), State::Active);
//     assert!(subject.terminated().is_none());

//     subject
//         .clone()
//         .on_termination(Termination::<Infallible>::Completed);
//     assert_eq!(checker.values(), [111]);
//     assert_eq!(checker.state(), State::Completed);
//     assert!(matches!(subject.terminated(), Some(Termination::Completed)));
// }

#[test]
fn test_error_on_next() {
    let mut subject = AsyncSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let _subscription = observable.clone().subscribe(observer);
    let mut subject_cloned = Some(subject.clone());
    let _subscription = observable.subscribe_with_callback(
        move |_| {
            subject_cloned
                .take()
                .unwrap()
                .on_termination(Termination::Error("error"));
        },
        move |_| {},
    );
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(subject.terminated().is_none());

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
    let mut subject: AsyncSubject<'_, _, Infallible> = AsyncSubject::default();
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
    *sub.lock_mut() = Some(
        observable
            .clone()
            .hook_on_next(move |observer, value| {
                if let Some(sub) = { sub_cloned.lock_mut().take() } {
                    sub.dispose();
                }
                observer.on_next(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock_mut() = Some(
        observable
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                if let Some(sub) = { sub_cloned.lock_mut().take() } {
                    sub.dispose();
                }
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert!(matches!(subject.terminated(), Some(Termination::Completed)));
}

#[test]
fn test_unsub_on_completed() {
    let mut subject: AsyncSubject<'_, _, Infallible> = AsyncSubject::default();
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
    *sub.lock_mut() = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                { sub_cloned.lock_mut().take() }.unwrap().dispose();
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock_mut() = Some(
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                { sub_cloned.lock_mut().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
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
fn test_unsub_on_error() {
    let mut subject = AsyncSubject::default();
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
    *sub.lock_mut() = Some(
        observable
            .clone()
            .hook_on_termination(move |observer, value| {
                { sub_cloned.lock_mut().take() }.unwrap().dispose();
                observer.on_termination(value);
            })
            .subscribe(observer_2),
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    *sub.lock_mut() = Some(
        observable
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                { sub_cloned.lock_mut().take() }.unwrap().dispose();
            })
            .subscribe(observer_3),
    );

    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.on_next(111);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), []);
    assert_eq!(checker_3.state(), State::Active);
    assert!(subject.terminated().is_none());

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), []);
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
        let subject = AsyncSubject::default();
        _subscription = subject.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = AsyncSubject::<'_, TestStruct, TestStruct>::default();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: AsyncSubject<'_, i32, String> = AsyncSubject::default();
    let observable = subject;

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: AsyncSubject<'_, i32, String> = AsyncSubject::default();
    let observable = subject;

    observable.filter(|_| true);
}
