mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::ReceiverObservable;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{just::Just, throw::Throw},
        error_handling::retry::{Retry, RetryAction},
    },
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_no_retry() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        let observable = source.clone();
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(errors.clone_value(), []);
}

#[test]
fn test_completed_retry_once() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        let observable = source.clone();
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(errors.clone_value(), ["error"]);
}

#[test]
fn test_completed_retry_twice() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        let observable = source.clone();
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    channels.on_termination(1, Termination::Error("error2"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Error("error2"));
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error", "error2"]);

    assert!(channels.on_next(2, 333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error", "error2"]);

    channels.on_termination(2, Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(2), ChannelState::Completed);
    assert_eq!(errors.clone_value(), ["error", "error2"]);
}

#[test]
fn test_completed_different_retry_observable() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = observable.retry(move |error| match error {
        -1 => RetryAction::Retry(
            Just::new(222)
                .into_shared()
                .with_error_type()
                .concat_with(Throw::new(0).into_shared().with_item_type())
                .into_send_boxed(),
        ),
        0 => RetryAction::Retry(
            Throw::new(1)
                .with_item_type()
                .into_shared()
                .into_send_boxed(),
        ),
        1 => RetryAction::Retry(
            Just::new(333)
                .with_error_type()
                .into_shared()
                .into_send_boxed(),
        ),
        _ => panic!(),
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(-1));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error(-1));
}

#[test]
fn test_completed_synchronous_throw() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        match error {
            -1 => RetryAction::Retry(
                Throw::new(0)
                    .with_item_type()
                    .into_shared()
                    .into_send_boxed(),
            ),
            0 => {
                let observable = source.clone();
                RetryAction::Retry(observable.into_send_boxed())
            }
            _ => panic!(),
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error(-1));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error(-1));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), [-1, 0]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), [-1, 0]);

    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(errors.clone_value(), [-1, 0]);
}

#[test]
fn test_erryr_no_retry() {
    let (checker, observer) = Checker::new();

    let (channels, observable) = test_channels();
    let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _, Shared>>::Stop);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
}

#[test]
fn test_error_retry_once() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 1 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    channels.on_termination(1, Termination::Error("error2"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error2"));
    assert_eq!(channels.state(1), ChannelState::Error("error2"));
    assert_eq!(errors.clone_value(), ["error", "error2"]);
}

#[test]
fn test_error_retry_twice() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 2 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    channels.on_termination(1, Termination::Error("error2"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Error("error2"));
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error", "error2"]);

    assert!(channels.on_next(2, 333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error", "error2"]);

    channels.on_termination(2, Termination::Error("error3"));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error3"));
    assert_eq!(channels.state(2), ChannelState::Error("error3"));
    assert_eq!(errors.clone_value(), ["error", "error2", "error3"]);
}

#[test]
fn test_error_source_and_retry_are_same() {
    let mut subject = PublishSubject::shared();
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let subject_cloned = subject.clone();
    let errors_cloned = errors.clone();
    let observable = subject.clone().retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 3 {
            RetryAction::Retry(subject_cloned.clone())
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(errors.with_ref(Vec::is_empty));

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(errors.clone_value(), ["error", "error", "error", "error"]);
}

#[test]
fn test_unsubscribe_before_retry() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error: String| {
        errors_cloned.with_mut(|values| values.push(error.clone()));
        if errors_cloned.with_ref(Vec::len) <= 1 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert!(errors.with_ref(Vec::is_empty));
}

#[test]
fn test_unsubscribe_after_retry() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 1 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(errors.clone_value(), ["error"]);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 1 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, &value_1).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error(&error));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), [&error]);

    assert!(channels.on_next(1, &value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), [&error]);

    channels.on_termination(1, Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channels.state(1), ChannelState::Error(&error));
    assert_eq!(errors.clone_value(), [&error, &error]);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let observable = observable.retry(move |error: &str| {
        assert_eq!(error, "error");
        let observable = source.clone();
        RetryAction::Retry(observable)
    });

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |_| {},
    );

    assert!(channels.on_next(0, &mut value_1).is_continue());
    channels.on_termination(0, Termination::Error("error"));
    assert!(channels.on_next(1, &mut value_2).is_continue());
    channels.on_termination(1, Termination::Completed);
    drop(subscription);
    drop(channels);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();
        let errors = Arc::new(Mutex::new(Vec::new()));

        let (channels, observable) = test_channels();
        let source = observable.clone();
        let errors_cloned = errors.clone();
        let observable = observable.retry(move |error| {
            errors_cloned.with_mut(|values| values.push(error));
            if errors_cloned.with_ref(Vec::len) <= 1 {
                let observable = source.clone();
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        });

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(errors.with_ref(Vec::is_empty));

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(0, 111).is_continue());
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(errors.with_ref(Vec::is_empty));

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                channels_cloned.on_termination(0, Termination::Error("error"));
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Error("error"));
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(errors.clone_value(), ["error"]);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(1, 222).is_continue());
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(errors.clone_value(), ["error"]);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                channels_cloned.on_termination(1, Termination::Error("error2"));
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error("error2"));
        assert_eq!(channels.state(1), ChannelState::Error("error2"));
        assert_eq!(errors.clone_value(), ["error", "error2"]);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let observable = subject.clone();
    let observable_cloned = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 2 {
            RetryAction::Retry(observable_cloned.clone())
        } else {
            RetryAction::Stop(error)
        }
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
    assert!(errors.with_ref(Vec::is_empty));

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(errors.with_ref(Vec::is_empty));

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(errors.clone_value(), ["error", "error", "error", "error"]);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (checker, observer) = Checker::<_, &str>::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable
        .retry(move |error| {
            errors_cloned.with_mut(|values| values.push(error));
            let observable = source.clone();
            RetryAction::Retry(observable)
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert!(errors.with_ref(Vec::is_empty));
}

#[test]
fn test_multiple_operation() {
    let (checker, observer) = Checker::new();
    let errors_1 = Arc::new(Mutex::new(Vec::new()));
    let errors_2 = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source_1 = observable.clone();
    let source_2 = observable.clone();
    let errors_cloned_1 = errors_1.clone();
    let errors_cloned_2 = errors_2.clone();
    let observable = observable
        .retry(move |error| {
            errors_cloned_1.with_mut(|values| values.push(error));
            if errors_cloned_1.with_ref(Vec::len) <= 1 {
                let observable = source_1.clone();
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        })
        .retry(move |error| {
            errors_cloned_2.with_mut(|values| values.push(error));
            if errors_cloned_2.with_ref(Vec::len) <= 1 {
                let observable = source_2.clone();
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors_1.with_ref(Vec::is_empty));
    assert!(errors_2.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors_1.with_ref(Vec::is_empty));
    assert!(errors_2.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors_1.clone_value(), ["error"]);
    assert!(errors_2.with_ref(Vec::is_empty));

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors_1.clone_value(), ["error"]);
    assert!(errors_2.with_ref(Vec::is_empty));

    channels.on_termination(1, Termination::Error("error2"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Error("error2"));
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors_1.clone_value(), ["error", "error2"]);
    assert_eq!(errors_2.clone_value(), ["error2"]);

    assert!(channels.on_next(2, 333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(errors_1.clone_value(), ["error", "error2"]);
    assert_eq!(errors_2.clone_value(), ["error2"]);

    channels.on_termination(2, Termination::Error("error3"));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error3"));
    assert_eq!(channels.state(2), ChannelState::Error("error3"));
    assert_eq!(errors_1.clone_value(), ["error", "error2"]);
    assert_eq!(errors_2.clone_value(), ["error2", "error3"]);
}

#[test]
fn test_without_convenient_api() {
    let (checker, observer) = Checker::new();
    let errors = Arc::new(Mutex::new(Vec::new()));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let errors_cloned = errors.clone();
    let observable = Retry::new(observable, move |error| {
        errors_cloned.with_mut(|values| values.push(error));
        if errors_cloned.with_ref(Vec::len) <= 1 {
            let observable = source.clone();
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(errors.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(errors.clone_value(), ["error"]);

    channels.on_termination(1, Termination::Error("error2"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error2"));
    assert_eq!(channels.state(1), ChannelState::Error("error2"));
    assert_eq!(errors.clone_value(), ["error", "error2"]);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<_, &str, _>::shared(111);
    let (checker, observer) = Checker::new();

    let observable = subject.clone().retry(move |error| {
        assert_eq!(error, "error");
        RetryAction::Retry(Just::new(222).with_error_type())
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

    let observable = Empty
        .with_error_type()
        .retry(move |_| RetryAction::<_, Throw<_>>::Stop("error"));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();

    let observable = Throw::new("error").retry(move |_| RetryAction::<_, Throw<_>>::Stop("error"));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. Retry hands the observer to its source, so the value still reaches it.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_continue());
        }))
    });

    let mut observable_1_option = Some(observable_1);
    let observable = observable
        .retry(move |_| RetryAction::Retry(observable_1_option.take().expect("retried once")));

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. Retry hands the observer to its source, so the completion still reaches it,
    // and nothing is retried, as a completion ends Retry.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let mut observable_1_option = Some(observable_1);
    let observable = observable
        .retry(move |_| RetryAction::Retry(observable_1_option.take().expect("retried once")));

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The retried source must not be subscribed after that, and the error must be
    // dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let mut observable_1_option = Some(observable_1);
    let observable = observable
        .retry(move |_| RetryAction::Retry(observable_1_option.take().expect("retried once")));

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(Just::new(1)).is_continue());
            observer.on_termination(Termination::Error("error"));
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _, Shared>>::Stop);

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
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, _, String>| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _, Shared>>::Stop);

        let (_, mut observer) = Checker::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.retry(RetryAction::<_, ReceiverObservable<'_, _, _>>::Stop);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, String>();
    let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _, Shared>>::Stop);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, Infallible>();
    let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _, Shared>>::Stop);

    observable.filter(|_| true);
}
