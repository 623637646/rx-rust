mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::safe_lock_vec;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, utility::do_after_termination::DoAfterTermination},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_after_termination(move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_completed_order() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let checker_1_cloned = checker_1.clone();
    let checker_2_cloned = checker_2.clone();
    let observable = observable.do_after_termination(move |termination| {
        assert_eq!(checker_1_cloned.state(), State::Completed);
        assert_eq!(checker_2_cloned.state(), State::Active);
        observer_2.on_termination(termination.clone());
        assert_eq!(checker_1_cloned.state(), State::Completed);
        assert_eq!(checker_2_cloned.state(), State::Completed);
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_after_termination(move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let terminations = Shared::new(Mutable::new(Vec::new()));
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.do_after_termination(|termination| {
        safe_lock_vec!(push: terminations, termination.clone());
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        terminations.test_lock_ref().as_ref(),
        vec![Termination::Error("error")]
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
    let observable = observable.do_after_termination(|termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(&value);
    assert_eq!(checker_1.values(), [&value]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error(&error));
    assert_eq!(checker_1.values(), [&value]);
    assert_eq!(checker_1.state(), State::Error(&error));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error(&error));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<(), _>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_after_termination(|termination| {
            observer_2.on_termination(termination.clone());
        });

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer_1) })
            .await
            .unwrap();
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let terminations = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let terminations_cloned = terminations.clone();
    let observable = observable.do_after_termination(move |termination| {
        safe_lock_vec!(push: terminations_cloned, termination.clone());
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        terminations.test_lock_ref().as_ref(),
        vec![Termination::Error("error"), Termination::Error("error")]
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = observable
        .do_after_termination(move |termination| {
            observer_2.on_termination(termination.clone());
        })
        .take(1);

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();
    let terminations = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let terminations_cloned_1 = terminations.clone();
    let terminations_cloned_2 = terminations.clone();
    let observable = observable
        .do_after_termination(move |termination| {
            safe_lock_vec!(push: terminations_cloned_1, termination.clone());
        })
        .do_after_termination(move |termination| {
            safe_lock_vec!(push: terminations_cloned_2, termination.clone());
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: terminations));

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(
        terminations.test_lock_ref().as_ref(),
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
    let observable = DoAfterTermination::new(observable, move |termination| {
        observer_2.on_termination(termination.clone());
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error("error"));
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

        let observable = observable.do_after_termination(|_| {});

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
        let observable = observable.do_after_termination(|_| {});

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
    let observable = observable.do_after_termination(|_| {
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
    let observable = observable.do_after_termination(|_| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_after_termination(|_| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.do_after_termination(|_| {});

    observable.filter(|_| true);
}
