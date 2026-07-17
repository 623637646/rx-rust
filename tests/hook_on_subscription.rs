mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::types::Shared;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, others::hook_on_subscription::HookOnSubscription},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Completed);
        }))
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
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
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        }))
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
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
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        }))
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Error(&error));
        }))
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let (mut sender, observable, channel_checker) = test_channel::<'_, &mut i32, Infallible>();

    // Custom operations
    let observable = observable.hook_on_subscription(|observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Completed);
        }))
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
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&mut value);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    drop(subscription);
    drop(channel_checker);

    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let channel_checker_cloned = channel_checker.clone();
        let observable = observable.hook_on_subscription(move |observable, observer| {
            assert_eq!(channel_checker_cloned.state(), ChannelState::Initialized);
            let sub = observable.subscribe(observer);
            assert_eq!(channel_checker_cloned.state(), ChannelState::Subscribed);
            sub.then(CallbackDisposal::new(move || {
                assert_eq!(channel_checker_cloned.state(), ChannelState::Unsubscribed);
            }))
        });

        let subscription = runtime
            .spawn(async { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move {
                sender.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let channel_checker = Shared::new(channel_checker);

    // Custom operations
    let observable = observable
        .hook_on_subscription(|observable, observer| {
            assert_eq!(channel_checker.state(), ChannelState::Initialized);
            let sub = observable.subscribe(observer);
            assert_eq!(channel_checker.state(), ChannelState::Subscribed);

            let channel_checker = channel_checker.clone();
            sub.then(CallbackDisposal::new(move || {
                assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
            }))
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .hook_on_subscription(|observable, observer| {
            assert_eq!(channel_checker.state(), ChannelState::Initialized);
            let sub = observable.subscribe(observer);
            assert_eq!(channel_checker.state(), ChannelState::Subscribed);
            sub.then(CallbackDisposal::new(|| {
                assert_eq!(channel_checker.state(), ChannelState::Completed);
            }))
        })
        .hook_on_subscription(|observable, observer| {
            assert_eq!(channel_checker.state(), ChannelState::Initialized);
            let sub = observable.subscribe(observer);
            assert_eq!(channel_checker.state(), ChannelState::Subscribed);
            sub.then(CallbackDisposal::new(|| {
                assert_eq!(channel_checker.state(), ChannelState::Completed);
            }))
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = HookOnSubscription::new(observable, |observable, observer| {
        assert_eq!(channel_checker.state(), ChannelState::Initialized);
        let sub = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        sub.then(CallbackDisposal::new(|| {
            assert_eq!(channel_checker.state(), ChannelState::Completed);
        }))
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
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
            Subscription::default()
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

    let _ = observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
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
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable =
        subject.hook_on_subscription(|observable, observer| observable.subscribe(observer));

    observable.filter(|_| true);
}
