mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::{ChannelChecker, ReceiverObservable, SenderObserver};
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::safe_lock_option;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{
        creating::{create::Create, just::Just, throw::Throw},
        error_handling::retry::{Retry, RetryAction},
    },
    subject::publish_subject::PublishSubject,
};
use rx_rust::{safe_lock_option_observer, safe_lock_vec};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

fn new_channel<'or, T, E>(
    sender: Shared<Mutable<Option<SenderObserver<'or, T, E>>>>,
    channel_checker: Shared<Mutable<Option<ChannelChecker<E>>>>,
) -> ReceiverObservable<'or, T, E> {
    let (sender_1, observable, channel_checker_1) = test_channel();
    safe_lock_option!(replace: sender, sender_1);
    safe_lock_option!(replace: channel_checker, channel_checker_1);
    observable
}

#[test]
fn test_completed_no_retry() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(
        safe_lock_option_observer!(on_termination: sender, Termination::<Infallible>::Completed)
    );
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
    assert_eq!(*errors.test_lock_ref(), []);
}

#[test]
fn test_completed_retry_once() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Completed));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);
}

#[test]
fn test_completed_retry_twice() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
        RetryAction::Retry(observable)
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error2")));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);

    assert!(safe_lock_option_observer!(on_next: sender, 333));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Completed));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);
}

#[test]
fn test_completed_different_retry_observable() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.retry(move |error| match error {
        -1 => RetryAction::Retry(
            Just::new(222)
                .with_error_type()
                .concat_with(Throw::new(0).with_item_type())
                .into_boxed(),
        ),
        0 => RetryAction::Retry(Throw::new(1).with_item_type().into_boxed()),
        1 => RetryAction::Retry(Just::new(333).with_error_type().into_boxed()),
        _ => panic!(),
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
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
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        match error {
            -1 => RetryAction::Retry(Throw::new(0).with_item_type().into_boxed()),
            0 => {
                let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
                RetryAction::Retry(observable.into_boxed())
            }
            _ => panic!(),
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error(-1)));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), [-1, 0]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), [-1, 0]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Completed));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Completed
    );
    assert_eq!(*errors.test_lock_ref(), [-1, 0]);
}

#[test]
fn test_erryr_no_retry() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error")
    );
}

#[test]
fn test_error_retry_once() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 1 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error2")));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error2"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error2")
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);
}

#[test]
fn test_error_retry_twice() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 2 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error2")));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);

    assert!(safe_lock_option_observer!(on_next: sender, 333));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error3")));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error3"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error3")
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2", "error3"]);
}

#[test]
fn test_error_source_and_retry_are_same() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let subject_cloned = subject.clone();
    let errors_cloned = errors.clone();
    let observable = subject.clone().retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 3 {
            RetryAction::Retry(subject_cloned.clone())
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: errors));

    subject.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: errors));

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(
        *errors.test_lock_ref(),
        ["error", "error", "error", "error"]
    );
}

#[test]
fn test_unsubscribe_before_retry() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error: String| {
        safe_lock_vec!(push: errors_cloned, error.clone());
        if safe_lock_vec!(len: errors_cloned) <= 1 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));
}

#[test]
fn test_unsubscribe_after_retry() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 1 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 1 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, &value_1));
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error(&error)));
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), [&error]);

    assert!(safe_lock_option_observer!(on_next: sender, &value_2));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), [&error]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error(&error)));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error(&error)
    );
    assert_eq!(*errors.test_lock_ref(), [&error, &error]);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;

    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = observable.retry(move |error: &str| {
        assert_eq!(error, "error");
        let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
        RetryAction::Retry(observable)
    });

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |_| {},
    );

    assert!(safe_lock_option_observer!(on_next: sender, &mut value_1));
    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert!(safe_lock_option_observer!(on_next: sender, &mut value_2));
    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Completed));
    drop(sender);
    drop(subscription);
    drop(channel_checker);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let sender = Shared::new(Mutable::new(None));
        let channel_checker = Shared::new(Mutable::new(None));
        let (checker, observer) = Checker::new();
        let errors = Shared::new(Mutable::new(Vec::new()));

        // Custom operations
        let observable = new_channel(sender.clone(), channel_checker.clone());
        let sender_cloned = sender.clone();
        let channel_checker_cloned = channel_checker.clone();
        let errors_cloned = errors.clone();
        let observable = observable.retry(move |error| {
            safe_lock_vec!(push: errors_cloned, error);
            if safe_lock_vec!(len: errors_cloned) <= 1 {
                let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        });

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );
        assert!(safe_lock_vec!(is_empty: errors));

        let sender = runtime
            .spawn(async move {
                assert!(safe_lock_option_observer!(on_next: sender, 111));
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );
        assert!(safe_lock_vec!(is_empty: errors));

        let sender = runtime
            .spawn(async move {
                assert!(
                    safe_lock_option_observer!(on_termination: sender, Termination::Error("error"))
                );
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );
        assert_eq!(*errors.test_lock_ref(), ["error"]);

        let sender = runtime
            .spawn(async move {
                assert!(safe_lock_option_observer!(on_next: sender, 222));
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Subscribed
        );
        assert_eq!(*errors.test_lock_ref(), ["error"]);

        let _sender = runtime
            .spawn(async move {
                assert!(
                    safe_lock_option_observer!(on_termination: sender, Termination::Error("error2"))
                );
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error("error2"));
        assert_eq!(
            channel_checker.test_lock_ref().as_ref().unwrap().state(),
            ChannelState::Error("error2")
        );
        assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let observable_cloned = observable.clone();
    let errors_cloned = errors.clone();
    let observable = observable.retry(move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 2 {
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
    assert!(safe_lock_vec!(is_empty: errors));

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(safe_lock_vec!(is_empty: errors));

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        *errors.test_lock_ref(),
        ["error", "error", "error", "error"]
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::<_, &str>::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = observable
        .retry(move |error| {
            safe_lock_vec!(push: errors_cloned, error);
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        })
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Unsubscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));
}

#[test]
fn test_multiple_operation() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors_1 = Shared::new(Mutable::new(Vec::new()));
    let errors_2 = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned_1 = sender.clone();
    let sender_cloned_2 = sender.clone();
    let channel_checker_cloned_1 = channel_checker.clone();
    let channel_checker_cloned_2 = channel_checker.clone();
    let errors_cloned_1 = errors_1.clone();
    let errors_cloned_2 = errors_2.clone();
    let observable = observable
        .retry(move |error| {
            safe_lock_vec!(push: errors_cloned_1, error);
            if safe_lock_vec!(len: errors_cloned_1) <= 1 {
                let observable =
                    new_channel(sender_cloned_1.clone(), channel_checker_cloned_1.clone());
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        })
        .retry(move |error| {
            safe_lock_vec!(push: errors_cloned_2, error);
            if safe_lock_vec!(len: errors_cloned_2) <= 1 {
                let observable =
                    new_channel(sender_cloned_2.clone(), channel_checker_cloned_2.clone());
                RetryAction::Retry(observable)
            } else {
                RetryAction::Stop(error)
            }
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors_1));
    assert!(safe_lock_vec!(is_empty: errors_2));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors_1));
    assert!(safe_lock_vec!(is_empty: errors_2));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors_1.test_lock_ref(), ["error"]);
    assert!(safe_lock_vec!(is_empty: errors_2));

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors_1.test_lock_ref(), ["error"]);
    assert!(safe_lock_vec!(is_empty: errors_2));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error2")));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors_1.test_lock_ref(), ["error", "error2"]);
    assert_eq!(*errors_2.test_lock_ref(), ["error2"]);

    assert!(safe_lock_option_observer!(on_next: sender, 333));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors_1.test_lock_ref(), ["error", "error2"]);
    assert_eq!(*errors_2.test_lock_ref(), ["error2"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error3")));
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Error("error3"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error3")
    );
    assert_eq!(*errors_1.test_lock_ref(), ["error", "error2"]);
    assert_eq!(*errors_2.test_lock_ref(), ["error2", "error3"]);
}

#[test]
fn test_without_convenient_api() {
    let sender = Shared::new(Mutable::new(None));
    let channel_checker = Shared::new(Mutable::new(None));
    let (checker, observer) = Checker::new();
    let errors = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = new_channel(sender.clone(), channel_checker.clone());
    let sender_cloned = sender.clone();
    let channel_checker_cloned = channel_checker.clone();
    let errors_cloned = errors.clone();
    let observable = Retry::new(observable, move |error| {
        safe_lock_vec!(push: errors_cloned, error);
        if safe_lock_vec!(len: errors_cloned) <= 1 {
            let observable = new_channel(sender_cloned.clone(), channel_checker_cloned.clone());
            RetryAction::Retry(observable)
        } else {
            RetryAction::Stop(error)
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_next: sender, 111));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert!(safe_lock_vec!(is_empty: errors));

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error")));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_next: sender, 222));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Subscribed
    );
    assert_eq!(*errors.test_lock_ref(), ["error"]);

    assert!(safe_lock_option_observer!(on_termination: sender, Termination::Error("error2")));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error2"));
    assert_eq!(
        channel_checker.test_lock_ref().as_ref().unwrap().state(),
        ChannelState::Error("error2")
    );
    assert_eq!(*errors.test_lock_ref(), ["error", "error2"]);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<'_, _, &str>::new(111);
    let (checker, observer) = Checker::new();

    // Custom operations
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

    // Custom operations
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

    // Custom operations
    let observable = Throw::new("error").retry(move |_| RetryAction::<_, Throw<_>>::Stop("error"));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
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
        let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

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
        let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

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
        let observable = observable.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

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
    let observable = observable.retry(RetryAction::<_, ReceiverObservable<'_, _, _>>::Stop);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, String> = PublishSubject::default();
    let observable = subject.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, Just<i32>, Infallible> = PublishSubject::default();
    let observable = subject.retry(RetryAction::<_, PublishSubject<'_, _, _>>::Stop);

    observable.filter(|_| true);
}
