mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::creating::create::Create,
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::<String>::Completed);
        Subscription::default()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_from_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Create::new(|observer| observable.subscribe(observer));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_from_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Create::new(|observer| observable.subscribe(observer));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let runtime_cloned = runtime.clone();
        let observable = Create::new(move |mut observer| {
            assert!(observer.on_next(1).is_continue());
            let runtime = runtime_cloned.clone();
            let handle = runtime_cloned.spawn_future(async move {
                runtime.sleep(DURATION_100_MS).await;
                assert!(observer.on_next(2).is_continue());
                runtime.sleep(DURATION_100_MS).await;
                assert!(observer.on_next(3).is_continue());
                runtime.sleep(DURATION_100_MS).await;
                observer.on_termination(Termination::<String>::Completed);
            });
            Subscription::new(CallbackDisposal::new(move || handle.dispose()))
        });
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [1]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [1]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [1]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [1, 2]);
        assert_eq!(checker_2.state(), State::Active);

        subscription_1.dispose(); // unsubscribe

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [1, 2, 3]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [1, 2]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [1, 2, 3]);
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsubscribe_wrap_observable() {
    let mut subject = PublishSubject::default();
    let subject_cloned = subject.clone();
    let observable = Create::new(|observer| subject_cloned.subscribe(observer));
    let (checker, observer) = Checker::new();

    let subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(&value).is_continue());
        observer.on_termination(Termination::Error(&error));
        Subscription::default()
    });
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        Subscription::default()
    });
    let (checker, observer) = Checker::new();

    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(*error));
                *error *= 2;
            }
        },
    );

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let runtime_cloned = runtime.clone();
        let observable = Create::new(move |mut observer| {
            assert!(observer.on_next(1).is_continue());
            let runtime = runtime_cloned.clone();
            let handle = runtime_cloned.spawn_future(async move {
                runtime.sleep(DURATION_100_MS).await;
                assert!(observer.on_next(2).is_continue());
                runtime.sleep(DURATION_100_MS).await;
                observer.on_termination(Termination::<String>::Completed);
            });
            Subscription::new(CallbackDisposal::new(move || handle.dispose()))
        });
        let (checker, observer) = Checker::new();

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [1, 2]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [1, 2]);
        assert_eq!(checker.state(), State::Dropped);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [1, 2]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable_1 = observable.clone();
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_stop());
        assert!(observer.on_next(222).is_stop());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    })
    .take(1);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
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
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

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

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    Create::new(|mut observer| {
        s.consume();
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });

    observable.filter(|_| true);
}
