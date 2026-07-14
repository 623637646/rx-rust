mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{
        creating::{create::Create, just::Just, throw::Throw},
        error_handling::catch::Catch,
    },
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |value| {
        assert_eq!(value, "error");
        observable_1
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_completed_without_catch() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (_sender_1, observable_1, channel_checker_1) = test_channel::<_, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |_| observable_1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |value| {
        assert_eq!(value, "error");
        observable_1
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error(true));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error(true));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Error(true));
}

#[test]
fn test_error_source_and_catch_are_same() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let subject_cloned = subject.clone();
    let observable = subject.clone().catch(move |value| {
        assert_eq!(value, "error");
        subject_cloned
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe_before_catch() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut _sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |_| observable_1);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_unsubscribe_after_catch() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |value| {
        assert_eq!(value, "error");
        observable_1
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |value| {
        assert_eq!(value, &error);
        observable_1
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(&value_1);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;

    let (mut sender, observable, _) = test_channel();
    let (mut sender_1, observable_1, _) = test_channel();

    // Custom operations
    let observable = observable.catch(move |error| {
        assert_eq!(error, "error");
        observable_1
    });

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |termination: Termination<&'static str>| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => assert_eq!(error, "error2"),
        },
    );

    sender.on_next(&mut value_1);
    sender.on_termination(Termination::Error("error"));
    sender_1.on_next(&mut value_2);
    sender_1.on_termination(Termination::Error("error2"));
    drop(subscription);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.catch(move |value| {
            assert_eq!(value, "error");
            observable_1
        });

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

        let sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

        runtime
            .spawn(async move {
                sender.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let sender_1 = runtime
            .spawn(async move {
                sender_1.on_next(222);
                sender_1
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move {
                sender_1.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let mut subject_1 = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let subject_1_cloned = subject_1.clone();
    let observable = observable.catch(move |value| {
        assert_eq!(value, "error");
        subject_1_cloned
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

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);

    subject_1.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);

    subject_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (_sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .catch(move |value: &str| {
            assert_eq!(value, "error");
            observable_1
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .catch(move |value| {
            assert_eq!(value, "error");
            observable_1
        })
        .catch(move |value| {
            assert_eq!(value, "error".to_string());
            observable_2
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::Error("error".to_string()));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        channel_checker_1.state(),
        ChannelState::Error("error".to_string())
    );
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        channel_checker_1.state(),
        ChannelState::Error("error".to_string())
    );
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    sender_2.on_termination(Termination::Error(0.1));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error(0.1));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        channel_checker_1.state(),
        ChannelState::Error("error".to_string())
    );
    assert_eq!(channel_checker_2.state(), ChannelState::Error(0.1));
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Catch::new(observable, move |value| {
        assert_eq!(value, "error");
        observable_1
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<'_, _, &str>::new(111);
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone().catch(move |value| {
        assert_eq!(value, "error");
        Just::new(222)
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Empty.catch(move |_| {
        if true {
            unreachable!();
        }
        Empty
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Throw::new("error").catch(move |error| {
        assert_eq!(error, "error");
        Throw::new("error")
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_race_condition_error_after_unsub() {
    let mut sender = None;
    let observable = Create::new(|observer| {
        sender = Some(observer);
        Subscription::default()
    });

    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.catch(move |value| {
        assert_eq!(value, "error");
        Just::new(222)
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    sender.as_mut().unwrap().on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.unwrap().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
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
            observer.on_next(Just::new(1));
            observer.on_termination(Termination::Error("error"));
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.catch(move |value| Throw::new(value).map_infallible_to_value());

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
        let observable = Create::new(|observer: BoxedObserver<'_, _, String>| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.catch(move |value| Throw::new(value).map_infallible_to_value());

        let (_, mut observer) = Checker::new();
        observer.on_next(&life_marker_2);
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub = TestStruct;
    let mut life_marker_or = None;

    // Error
    // let mut life_marker_or = None;
    // let life_marker_sub = TestStruct;

    {
        let observable = Create::new(
            |observer: BoxedObserver<'_, Just<&TestStruct>, Infallible>| {
                life_marker_or = Some(observer);
                Subscription::new(CallbackDisposal::new(|| {
                    life_marker_sub.consume_ref();
                }))
            },
        );
        let observable = observable.catch(move |value| Throw::new(value).map_infallible_to_value());

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.catch(move |value| Throw::new(value).map_infallible_to_value());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, String> = PublishSubject::default();
    let observable = subject.catch(move |value| Throw::new(value).map_infallible_to_value());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, Infallible> = PublishSubject::default();
    let observable = subject.catch(move |value| Throw::new(value).map_infallible_to_value());

    observable.filter(|_| true);
}
