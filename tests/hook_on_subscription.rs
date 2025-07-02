mod tests_utils;

use crate::tests_utils::test_runtime::{block_on, spawn};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, others::hook_on_subscription::HookOnSubscription},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_completed());
        })
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_error("error"));
        })
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_unsubscribed());
        })
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_error(&error));
        })
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_error(&error));
    assert!(channel_checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, Infallible>();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_completed());
        })
    });

    let subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => {}
            Termination::Error(_) => panic!(),
        },
    );
    assert!(channel_checker.is_subscribed());

    sender.on_next(&mut value);
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert!(channel_checker.is_completed());

    drop(subscription);
    drop(channel_checker);

    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let channel_checker_cloned = channel_checker.clone();
        let observable = observable.hook_on_subscription(move |observable, observer| {
            assert!(channel_checker_cloned.is_initialized());
            let sub = observable.subscribe(observer);
            assert!(channel_checker_cloned.is_subscribed());
            sub + Subscription::new_with_disposal_callback(move || {
                assert!(channel_checker_cloned.is_unsubscribed());
            })
        });

        let handle = spawn(async { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let handle = spawn(async move {
            sender.on_next(111);
        });
        handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let handle = spawn(async { subscription.dispose() });
        handle.await.unwrap();
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable =
        observable.hook_on_subscription(move |observable, observer| observable.subscribe(observer));
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_error("error"));
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .hook_on_subscription(|observable, observer| {
            assert!(channel_checker.is_initialized());
            let sub = observable.subscribe(observer);
            assert!(channel_checker.is_subscribed());
            sub + Subscription::new_with_disposal_callback(|| {
                assert!(channel_checker.is_completed());
            })
        })
        .hook_on_subscription(|observable, observer| {
            assert!(channel_checker.is_initialized());
            let sub = observable.subscribe(observer);
            assert!(channel_checker.is_subscribed());
            sub + Subscription::new_with_disposal_callback(|| {
                assert!(channel_checker.is_completed());
            })
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = HookOnSubscription::new(observable, |observable, observer| {
        assert!(channel_checker.is_initialized());
        let sub = observable.subscribe(observer);
        assert!(channel_checker.is_subscribed());
        sub + Subscription::new_with_disposal_callback(|| {
            assert!(channel_checker.is_completed());
        })
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
    assert!(channel_checker.is_completed());
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

        let observable =
            observable.hook_on_subscription(|observable, observer| observable.subscribe(observer));

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
        let observable =
            observable.hook_on_subscription(|observable, observer| observable.subscribe(observer));

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
    let observable = observable.hook_on_subscription(|observable, observer| {
        s.consume();
        observable.subscribe(observer)
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
    let observable =
        observable.hook_on_subscription(|observable, observer| observable.subscribe(observer));
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable =
        subject.hook_on_subscription(|observable, observer| observable.subscribe(observer));

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable =
        subject.hook_on_subscription(|observable, observer| observable.subscribe(observer));

    observable.filter(|_| true);
}
