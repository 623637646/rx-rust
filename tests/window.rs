mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, transforming::window::Window},
    subject::publish_subject::PublishSubject,
};
use rx_rust::{safe_lock, safe_lock_vec};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_from_source() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_from_boundary() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Completed);

    sender.on_next(444);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [444]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [444]);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_source_and_boundary_are_same() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone().window(subject.clone());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [()]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [()]);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_error_from_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Error("error"));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_boundary() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (boundary_sender, boundary_observable, boundary_channel_checker) =
        test_channel::<'_, (), &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| termination_observer.on_termination(termination),
    );

    sender.on_next(111);
    boundary_sender.on_termination(Termination::Error("boundary error"));

    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    let (checker, _) = &checker_sub_vec.test_lock_ref()[0];
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("boundary error"));
    assert_eq!(termination_checker.state(), State::Error("boundary error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(
        boundary_channel_checker.state(),
        ChannelState::Error("boundary error")
    );
}

#[test]
fn test_error_from_boundary_while_window_pending() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (boundary_sender, boundary_observable, boundary_channel_checker) =
        test_channel::<'_, (), &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The value is buffered because window 1 has no inner observer yet.
    sender.on_next(111);

    // The boundary errors while window 1 is still unsubscribed.
    boundary_sender.on_termination(Termination::Error("boundary error"));
    assert_eq!(termination_checker.state(), State::Error("boundary error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(
        boundary_channel_checker.state(),
        ChannelState::Error("boundary error")
    );

    // A late subscriber observes the buffered value and then the error.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("boundary error"));
}

#[test]
fn test_error_from_boundary_while_no_window_subscribed() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (boundary_sender, boundary_observable, boundary_channel_checker) =
        test_channel::<'_, (), &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );

    // Subscribe to window 1 and then unsubscribe, leaving no window to accept values.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let sub_1 = window_1.subscribe(observer_1);
    sender.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    drop(sub_1);
    // The window holds the observer between two events, so it is released by the next thing that
    // happens to the window, which is its end below.
    assert_eq!(checker_1.state(), State::Active);

    // The boundary error terminates the outer observable only: the unsubscribed
    // window has no observer left to receive it.
    boundary_sender.on_termination(Termination::Error("boundary error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(termination_checker.state(), State::Error("boundary error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(
        boundary_channel_checker.state(),
        ChannelState::Error("boundary error")
    );
}

#[test]
fn test_completed_boundary_does_not_mask_later_source_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Completed);

    sender.on_next(444);
    sender.on_termination(Termination::Error("error"));
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [444]);
                assert_eq!(checker.state(), State::Error("error"));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _sub)) in safe_lock!(mem_take: checker_sub_vec)
        .into_iter()
        .enumerate()
    {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Dropped);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value_1);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value_2);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_2]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(&value_3);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Error(&error));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
        let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
        let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

        // Custom operations
        let observable = observable.window(boundary_observable);

        let checker_sub_vec_cloned = checker_sub_vec.clone();
        let _subscription = runtime
            .spawn(async move {
                observable.subscribe_with_callback(
                    move |value| {
                        let (checker, observer) = Checker::new();
                        let sub = value.subscribe(observer);
                        safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
                    },
                    |termination| {
                        termination_observer.on_termination(termination);
                    },
                )
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let mut boundary_sender = runtime
            .spawn(async move {
                boundary_sender.on_next(());
                boundary_sender
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [222]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let sender = runtime
            .spawn(async move {
                sender.on_next(333);
                sender
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        let _boundary_sender = runtime
            .spawn(async move {
                boundary_sender.on_next(());
                boundary_sender
            })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert_eq!(checker.state(), State::Completed);
                }
                2 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move { sender.on_termination(Termination::Completed) })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert_eq!(checker.state(), State::Completed);
                }
                2 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Completed);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
    let mut boundary_subject = PublishSubject::default();
    let (termination_checker_1, termination_observer_1) = Checker::<Infallible, _>::new();
    let checker_sub_vec_1 = Shared::new(Mutable::new(Vec::new()));
    let (termination_checker_2, termination_observer_2) = Checker::<Infallible, _>::new();
    let checker_sub_vec_2 = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window(boundary_subject.clone());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let checker_sub_vec_cloned = checker_sub_vec_1.clone();
    let _subscription_1 = observable_1.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer_1.on_termination(termination);
        },
    );
    let checker_sub_vec_cloned = checker_sub_vec_2.clone();
    let _subscription_2 = observable_2.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer_2.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 1);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 1);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 1);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 1);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 2);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 2);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 2);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 2);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 2);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 2);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 3);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 3);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 3);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 3);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (_boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(boundary_observable).take(1);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Dropped);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender_1, boundary_observable_1, boundary_channel_checker_1) = test_channel();
    let (mut boundary_sender_2, boundary_observable_2, boundary_channel_checker_2) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let context = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable
        .window(boundary_observable_1)
        .window(boundary_observable_2);

    let context_cloned = context.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));
            let checker_sub_vec_cloned = checker_sub_vec.clone();
            let sub = value.subscribe_with_callback(
                move |value| {
                    let (checker, observer) = Checker::new();
                    let sub = value.subscribe(observer);
                    safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
                },
                |_| {},
            );
            safe_lock_vec!(push: context_cloned, (checker_sub_vec, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: context), 1);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: context), 1);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    boundary_sender_1.on_next(());
    assert_eq!(safe_lock_vec!(len: context), 1);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    boundary_sender_2.on_next(());
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 0);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 0);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    boundary_sender_1.on_next(());
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker_2.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(
        boundary_channel_checker_1.state(),
        ChannelState::Unsubscribed
    );
    assert_eq!(
        boundary_channel_checker_2.state(),
        ChannelState::Unsubscribed
    );
}

#[test]
fn test_multiple_operation_same_boundary() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let mut boundary_subject = PublishSubject::default();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let context = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable
        .window(boundary_subject.clone())
        .window(boundary_subject.clone());

    let context_cloned = context.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));
            let checker_sub_vec_cloned = checker_sub_vec.clone();
            let sub = value.subscribe_with_callback(
                move |value| {
                    let (checker, observer) = Checker::new();
                    let sub = value.subscribe(observer);
                    safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
                },
                |_| {},
            );
            safe_lock_vec!(push: context_cloned, (checker_sub_vec, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: context), 1);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: context), 1);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    boundary_subject.on_next(());
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: context), 2);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    boundary_subject.on_next(());
    assert_eq!(safe_lock_vec!(len: context), 3);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: context), 3);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert_eq!(checker.state(), State::Active);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: context), 3);
    for (index, (checker, _)) in context.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(safe_lock_vec!(len: checker), 1);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Window::new(observable, boundary_observable);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_revert_completed() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window(boundary_subject.clone());
    let observable_1 = observable.clone().merge_all();
    let observable_2 = observable.clone().concat_all();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Completed);
}

#[test]
fn test_revert_error() {
    let mut subject = PublishSubject::default();
    let mut boundary_subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window(boundary_subject.clone());
    let observable_1 = observable.clone().merge_all();
    let observable_2 = observable.clone().concat_all();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [111, 222]);
    assert_eq!(checker_3.state(), State::Error("error"));
}

#[test]
fn test_next_on_sub() {
    let mut subject = BehaviorSubject::new(111);
    let boundary_subject = BehaviorSubject::new(());
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone().window(boundary_subject.clone());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Empty.window(Empty.with_item_type());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Throw::new("error").window(Empty.with_item_type().with_error_type());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Error("error"));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error("error"));
}

#[test]
fn test_error_on_sub_from_boundary() {
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window(Throw::new("boundary error").with_item_type());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );

    // The boundary errors during its own subscription, before the source is subscribed.
    // The window opened up front is still emitted, and it observes the error.
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Error("boundary error"));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error("boundary error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_subscribe_stale_window_observable() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, _boundary_channel_checker) = test_channel();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Collect the window observables without subscribing to them immediately.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The boundary closes window 1 (still unsubscribed) and opens window 2.
    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: window_vec), 2);

    let mut windows = window_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let window_2 = windows.pop().unwrap();
    let window_1 = windows.pop().unwrap();

    // The current window (window 2) receives source values.
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    sender.on_next(222);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);

    // Window 1 already completed when the boundary fired, so its late subscriber
    // observes the completion immediately and receives no values.
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);

    // Source values keep flowing to the current window's subscriber only.
    sender.on_next(333);
    assert_eq!(checker_2.values(), [222, 333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_1.values(), []);
}

#[test]
fn test_subscribe_window_observable_after_termination() {
    use rx_rust::utils::types::MutableHelper;

    let (sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The source terminates while window 1 is still unsubscribed. The window
    // itself was completed by the source termination.
    sender.on_termination(Termination::Completed);
    assert_eq!(termination_checker.state(), State::Completed);

    // A late subscriber observes the completion.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);
}

#[test]
fn test_subscribe_stale_window_observable_after_error() {
    use rx_rust::utils::types::MutableHelper;

    let (sender, observable, _channel_checker) = test_channel::<'_, i32, &str>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The source errors while window 1 is still unsubscribed. The window itself
    // receives the same error.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(termination_checker.state(), State::Error("error"));

    // A late subscriber observes the error.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Error("error"));
}

#[test]
fn test_subscribe_boundary_closed_window_after_later_error() {
    use rx_rust::utils::types::MutableHelper;

    let (sender, observable, _channel_checker) = test_channel::<'_, i32, &str>();
    let (mut boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), &str>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold both window observables without subscribing to them immediately.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The boundary completes window 1 and opens window 2. The later source
    // error belongs to window 2 and must not change window 1's termination.
    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: window_vec), 2);
    sender.on_termination(Termination::Error("error"));
    assert_eq!(termination_checker.state(), State::Error("error"));

    let mut windows = window_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let window_2 = windows.pop().unwrap();
    let window_1 = windows.pop().unwrap();

    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);

    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_subscribe_window_observable_after_unsubscribe() {
    use rx_rust::utils::types::MutableHelper;

    let (_sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // Unsubscribing is not a termination: the window never completed nor errored.
    drop(subscription);

    // A late subscriber is dropped without receiving a termination.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
}

#[test]
fn test_subscribe_multiple_stale_window_observables() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, _boundary_channel_checker) = test_channel();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observables without subscribing to them.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );

    // Two boundaries: windows 1 and 2 are closed unobserved, window 3 is current.
    boundary_sender.on_next(());
    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: window_vec), 3);

    let mut windows = window_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let window_3 = windows.pop().unwrap();
    let window_2 = windows.pop().unwrap();
    let window_1 = windows.pop().unwrap();

    // The current window receives source values.
    let (checker_3, observer_3) = Checker::new();
    let _sub_3 = window_3.subscribe(observer_3);
    sender.on_next(333);
    assert_eq!(checker_3.values(), [333]);

    // Every stale window observes its completion immediately — even the one that
    // is several boundaries old — and none of them steals the current window.
    let (checker_1, observer_1) = Checker::new();
    let sub_1 = window_1.subscribe(observer_1);
    let (checker_2, observer_2) = Checker::new();
    let sub_2 = window_2.subscribe(observer_2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Completed);

    // Disposing a stale-window subscription is a no-op for the pipeline.
    drop(sub_1);
    drop(sub_2);
    sender.on_next(444);
    assert_eq!(checker_3.values(), [333, 444]);
    assert_eq!(checker_3.state(), State::Active);
}

#[test]
fn test_subscribe_current_window_late_with_earlier_values() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );

    // Values emitted before the window is subscribed are sent.
    sender.on_next(111);

    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111]);

    // Values emitted after subscribing are delivered.
    sender.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);
}

#[test]
fn test_subscribe_current_window_after_buffered_values_and_completion() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the current window without subscribing to it immediately.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // Values emitted before the delayed subscription are buffered, and source
    // completion must not discard them.
    sender.on_next(111);
    sender.on_next(222);
    sender.on_termination(Termination::Completed);

    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
}

#[test]
fn test_reentrant_source_value_during_buffer_replay() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary);

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // Buffer multiple values before subscribing to the current window so a
    // reentrant source value cannot overtake values still being replayed.
    source.on_next(111);
    source.on_next(222);

    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let mut source_reentrant = source.clone();
    let _inner_subscription = window_1.subscribe_with_callback(
        move |value| {
            let should_reenter = value == 111;
            safe_lock_vec!(push: values_cloned, value);
            if should_reenter {
                source_reentrant.on_next(333);
            }
        },
        |_termination| {},
    );

    // The reentrant source value still belongs to window 1, but must be
    // delivered only after all previously buffered values have been replayed.
    assert_eq!(safe_lock!(clone: values), [111, 222, 333]);
}

#[test]
fn test_disposing_outer_subscription_during_buffer_replay_suppresses_remaining_values() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary);

    let windows = Shared::new(Mutable::new(Vec::new()));
    let windows_cloned = windows.clone();
    let outer_subscription = Shared::new(Mutable::new(None));
    let subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: windows_cloned, window),
        |_termination| {},
    );
    outer_subscription.lock_mut(|mut lock| *lock = Some(subscription));

    source.on_next(111);
    source.on_next(222);

    let window = windows.lock_mut(|mut lock| lock.pop()).unwrap();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let outer_subscription_cloned = outer_subscription.clone();
    let _inner_subscription = window.subscribe_with_callback(
        move |value| {
            safe_lock_vec!(push: values_cloned, value);
            if value == 111 {
                let subscription = outer_subscription_cloned.lock_mut(|mut lock| lock.take());
                drop(subscription);
            }
        },
        |_termination| {},
    );

    assert_eq!(safe_lock!(clone: values), [111]);
}

#[test]
fn test_reentrant_boundary_during_buffer_replay_keeps_the_order_of_the_old_window() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary.clone());

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let events = Shared::new(Mutable::new(Vec::new()));
    let events_cloned = events.clone();
    let window_index = Shared::new(Mutable::new(0usize));
    let window_index_cloned = window_index.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_index_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            if index == 1 {
                safe_lock_vec!(push: events_cloned, "window_2_emitted");
            }
            safe_lock_vec!(push: window_vec_cloned, window);
        },
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // Buffer multiple values before subscribing to window 1 so a reentrant
    // boundary cannot terminate the window before replay finishes.
    source.on_next(111);
    source.on_next(222);

    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let values_1 = Shared::new(Mutable::new(Vec::new()));
    let values_1_cloned = values_1.clone();
    let terminations_1 = Shared::new(Mutable::new(Vec::<Termination<Infallible>>::new()));
    let terminations_1_cloned = terminations_1.clone();
    let events_values = events.clone();
    let events_termination = events.clone();
    let mut boundary_reentrant = boundary.clone();
    let _sub_1 = window_1.subscribe_with_callback(
        move |value| {
            let should_reenter = value == 111;
            safe_lock_vec!(push: values_1_cloned, value);
            safe_lock_vec!(
                push: events_values,
                match value {
                    111 => "window_1_value_111",
                    222 => "window_1_value_222",
                    _ => unreachable!(),
                }
            );
            if should_reenter {
                boundary_reentrant.on_next(());
            }
        },
        move |termination| {
            safe_lock_vec!(push: terminations_1_cloned, termination);
            safe_lock_vec!(push: events_termination, "window_1_completed");
        },
    );

    assert_eq!(safe_lock!(clone: values_1), [111, 222]);
    assert_eq!(safe_lock!(clone: terminations_1), [Termination::Completed]);
    // Each window is serialized on its own, independently of the outer Observable, so the
    // re-entrant boundary emits the new window while window 1 is still replaying its buffer. The
    // events of window 1 keep their order among themselves, and its completion stays last.
    assert_eq!(
        safe_lock!(clone: events),
        [
            "window_1_value_111",
            "window_2_emitted",
            "window_1_value_222",
            "window_1_completed",
        ]
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    // The new window remains independent and receives subsequent source values.
    let window_2 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    source.on_next(333);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
}

#[test]
fn test_boundary_completes_current_window_before_emitting_next_window() {
    use rx_rust::utils::types::MutableHelper;

    let source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.window(boundary.clone());

    let events = Shared::new(Mutable::new(Vec::new()));
    let window_index = Shared::new(Mutable::new(0usize));
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let events_cloned = events.clone();
    let window_index_cloned = window_index.clone();
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_index_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            if index == 1 {
                safe_lock_vec!(push: events_cloned, "window_2_emitted");
            }

            let events_termination = events_cloned.clone();
            let sub = window.subscribe_with_callback(
                |_value| {},
                move |termination| {
                    if index == 0 {
                        assert_eq!(termination, Termination::Completed);
                        safe_lock_vec!(push: events_termination, "window_1_completed");
                    }
                },
            );
            safe_lock_vec!(push: inner_subscriptions_cloned, sub);
        },
        |_termination| {},
    );

    boundary.on_next(());
    assert_eq!(
        safe_lock!(clone: events),
        ["window_1_completed", "window_2_emitted"]
    );
}

#[test]
fn test_subscribing_inner_after_outer_termination_is_queued_observes_termination() {
    let source: PublishSubject<'_, i32, &'static str> = PublishSubject::default();
    let mut boundary: PublishSubject<'_, (), &'static str> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary.clone());

    let (checker_2, observer_2) = Checker::new();
    let mut observer_2 = Some(observer_2);
    let mut source_reentrant = Some(source.clone());
    let mut window_index = 0usize;
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let outer_terminations = Shared::new(Mutable::new(Vec::new()));
    let outer_terminations_cloned = outer_terminations.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_index;
            window_index += 1;
            let sub = match index {
                0 => window.subscribe_with_callback(|_value| {}, |_termination| {}),
                1 => {
                    // Queue the outer termination while the delegate is still
                    // emitting window 2, then subscribe to that emitted window.
                    source_reentrant
                        .take()
                        .unwrap()
                        .on_termination(Termination::Error("error"));
                    window.subscribe(observer_2.take().unwrap())
                }
                _ => unreachable!(),
            };
            safe_lock_vec!(push: inner_subscriptions_cloned, sub);
        },
        move |termination| safe_lock_vec!(push: outer_terminations_cloned, termination),
    );

    boundary.on_next(());

    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        safe_lock!(clone: outer_terminations),
        [Termination::Error("error")]
    );
}

#[test]
fn test_disposing_outer_subscription_from_old_window_completion_suppresses_new_window() {
    use rx_rust::utils::types::MutableHelper;

    let source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.window(boundary.clone());

    let outer_subscription = Shared::new(Mutable::new(None));
    let outer_subscription_cloned = outer_subscription.clone();
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let window_count = Shared::new(Mutable::new(0usize));
    let window_count_cloned = window_count.clone();
    let subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_count_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            if index == 0 {
                let outer_subscription = outer_subscription_cloned.clone();
                let sub = window.subscribe_with_callback(
                    |_value| {},
                    move |termination| {
                        assert_eq!(termination, Termination::Completed);
                        let subscription = outer_subscription.lock_mut(|mut lock| lock.take());
                        drop(subscription);
                    },
                );
                safe_lock_vec!(push: inner_subscriptions_cloned, sub);
            }
        },
        |_termination| {},
    );
    outer_subscription.lock_mut(|mut lock| *lock = Some(subscription));

    boundary.on_next(());

    assert_eq!(safe_lock!(clone: window_count), 1);
}

#[test]
fn test_disposing_outer_subscription_from_inner_termination_suppresses_outer_termination() {
    use rx_rust::utils::types::MutableHelper;

    let source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary);

    let outer_subscription = Shared::new(Mutable::new(None));
    let outer_subscription_cloned = outer_subscription.clone();
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let inner_terminations = Shared::new(Mutable::new(Vec::new()));
    let inner_terminations_cloned = inner_terminations.clone();
    let outer_terminations = Shared::new(Mutable::new(Vec::new()));
    let outer_terminations_cloned = outer_terminations.clone();
    let subscription = observable.subscribe_with_callback(
        move |window| {
            let outer_subscription = outer_subscription_cloned.clone();
            let inner_terminations = inner_terminations_cloned.clone();
            let sub = window.subscribe_with_callback(
                |_value| {},
                move |termination| {
                    safe_lock_vec!(push: inner_terminations, termination);
                    let subscription = outer_subscription.lock_mut(|mut lock| lock.take());
                    drop(subscription);
                },
            );
            safe_lock_vec!(push: inner_subscriptions_cloned, sub);
        },
        move |termination| safe_lock_vec!(push: outer_terminations_cloned, termination),
    );
    outer_subscription.lock_mut(|mut lock| *lock = Some(subscription));

    source.on_termination(Termination::Completed);

    assert_eq!(
        safe_lock!(clone: inner_terminations),
        [Termination::Completed]
    );
    assert_eq!(safe_lock_vec!(len: outer_terminations), 0);
}

#[test]
fn test_disposing_inner_subscription_after_termination_is_queued_suppresses_inner_termination() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    // Custom operations
    let observable = source.clone().window(boundary);

    let inner_subscription = Shared::new(Mutable::new(None));
    let inner_subscription_outer = inner_subscription.clone();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let inner_terminations = Shared::new(Mutable::new(Vec::new()));
    let inner_terminations_cloned = inner_terminations.clone();
    let outer_terminations = Shared::new(Mutable::new(Vec::new()));
    let outer_terminations_cloned = outer_terminations.clone();
    let source_termination = Shared::new(Mutable::new(Some(source.clone())));
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let inner_subscription = inner_subscription_outer.clone();
            let inner_subscription_cloned = inner_subscription.clone();
            let values = values_cloned.clone();
            let inner_terminations = inner_terminations_cloned.clone();
            let source_termination = source_termination.clone();
            let sub = window.subscribe_with_callback(
                move |value| {
                    safe_lock_vec!(push: values, value);

                    // Queue the window termination, then dispose the inner
                    // subscription before that termination can be delivered.
                    let source = source_termination.lock_mut(|mut lock| lock.take()).unwrap();
                    source.on_termination(Termination::Completed);
                    let sub = inner_subscription_cloned.lock_mut(|mut lock| lock.take());
                    drop(sub);
                },
                move |termination| safe_lock_vec!(push: inner_terminations, termination),
            );
            inner_subscription_outer.lock_mut(|mut lock| *lock = Some(sub));
        },
        move |termination| safe_lock_vec!(push: outer_terminations_cloned, termination),
    );

    source.on_next(111);

    assert_eq!(safe_lock!(clone: values), [111]);
    assert_eq!(safe_lock_vec!(len: inner_terminations), 0);
    assert_eq!(
        safe_lock!(clone: outer_terminations),
        [Termination::Completed]
    );
}

#[test]
fn test_disposing_inner_subscription_after_boundary_rollover_is_queued_suppresses_old_termination()
{
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    let observable = source.clone().window(boundary.clone());

    let first_inner_subscription = Shared::new(Mutable::new(None));
    let first_inner_subscription_outer = first_inner_subscription.clone();
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let first_terminations = Shared::new(Mutable::new(Vec::new()));
    let first_terminations_cloned = first_terminations.clone();
    let second_values = Shared::new(Mutable::new(Vec::new()));
    let second_values_cloned = second_values.clone();
    let window_count = Shared::new(Mutable::new(0usize));
    let window_count_cloned = window_count.clone();
    let boundary_reentrant = Shared::new(Mutable::new(Some(boundary)));
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_count_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            match index {
                0 => {
                    let first_inner_subscription = first_inner_subscription_outer.clone();
                    let first_inner_subscription_cloned = first_inner_subscription.clone();
                    let first_terminations = first_terminations_cloned.clone();
                    let boundary_reentrant = boundary_reentrant.clone();
                    let sub = window.subscribe_with_callback(
                        move |_value| {
                            // Queue completion of the old window and emission of
                            // the new one, then cancel the old inner subscription.
                            boundary_reentrant
                                .lock_mut(|mut lock| lock.take())
                                .unwrap()
                                .on_next(());
                            let sub =
                                first_inner_subscription_cloned.lock_mut(|mut lock| lock.take());
                            drop(sub);
                        },
                        move |termination| safe_lock_vec!(push: first_terminations, termination),
                    );
                    first_inner_subscription.lock_mut(|mut lock| *lock = Some(sub));
                }
                1 => {
                    let second_values = second_values_cloned.clone();
                    let sub = window.subscribe_with_callback(
                        move |value| safe_lock_vec!(push: second_values, value),
                        |_termination| {},
                    );
                    safe_lock_vec!(push: inner_subscriptions_cloned, sub);
                }
                _ => unreachable!(),
            }
        },
        |_termination| {},
    );

    source.on_next(111);
    source.on_next(222);

    assert_eq!(safe_lock!(clone: window_count), 2);
    assert_eq!(safe_lock_vec!(len: first_terminations), 0);
    assert_eq!(safe_lock!(clone: second_values), [222]);
}

#[test]
fn test_disposing_inner_subscription_after_value_is_queued_suppresses_value() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    let observable = source.clone().window(boundary.clone());

    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let second_values = Shared::new(Mutable::new(Vec::new()));
    let second_values_cloned = second_values.clone();
    let window_count = Shared::new(Mutable::new(0usize));
    let window_count_cloned = window_count.clone();
    let mut source_reentrant = source.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_count_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            match index {
                0 => {
                    let sub = window.subscribe_with_callback(|_value| {}, |_termination| {});
                    safe_lock_vec!(push: inner_subscriptions_cloned, sub);
                }
                1 => {
                    let second_values = second_values_cloned.clone();
                    let sub = window.subscribe_with_callback(
                        move |value| safe_lock_vec!(push: second_values, value),
                        |_termination| {},
                    );

                    // Attach, forwarding this value, and detaching are all queued
                    // behind the current EmitWindow callback. Disposal must make
                    // the queued value unobservable before the queue drains.
                    source_reentrant.on_next(111);
                    drop(sub);
                }
                _ => unreachable!(),
            }
        },
        |_termination| {},
    );

    boundary.on_next(());
    source.on_next(222);

    assert_eq!(safe_lock!(clone: window_count), 2);
    assert_eq!(safe_lock_vec!(len: second_values), 0);
}

#[test]
fn test_disposing_inner_subscription_before_queued_attach_suppresses_buffered_values() {
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    let observable = source.clone().window(boundary.clone());

    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let second_values = Shared::new(Mutable::new(Vec::new()));
    let second_values_cloned = second_values.clone();
    let window_count = Shared::new(Mutable::new(0usize));
    let window_count_cloned = window_count.clone();
    let mut source_reentrant = source.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_count_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            match index {
                0 => {
                    let sub = window.subscribe_with_callback(|_value| {}, |_termination| {});
                    safe_lock_vec!(push: inner_subscriptions_cloned, sub);
                }
                1 => {
                    // Buffer a value before subscribing. The attach is queued behind
                    // the current EmitWindow callback, so subscribe returns before
                    // the buffered value can be replayed.
                    source_reentrant.on_next(111);

                    let second_values = second_values_cloned.clone();
                    let sub = window.subscribe_with_callback(
                        move |value| safe_lock_vec!(push: second_values, value),
                        |_termination| {},
                    );
                    drop(sub);
                }
                _ => unreachable!(),
            }
        },
        |_termination| {},
    );

    boundary.on_next(());
    source.on_next(222);

    assert_eq!(safe_lock!(clone: window_count), 2);
    assert_eq!(safe_lock_vec!(len: second_values), 0);
}

#[test]
fn test_disposing_old_inner_during_multiple_queued_boundary_rollovers_keeps_latest_window_working()
{
    use rx_rust::utils::types::MutableHelper;

    let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
    let boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();

    let observable = source.clone().window(boundary.clone());

    let first_inner_subscription = Shared::new(Mutable::new(None));
    let first_inner_subscription_outer = first_inner_subscription.clone();
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let first_terminations = Shared::new(Mutable::new(Vec::new()));
    let first_terminations_cloned = first_terminations.clone();
    let second_terminations = Shared::new(Mutable::new(Vec::new()));
    let second_terminations_cloned = second_terminations.clone();
    let latest_values = Shared::new(Mutable::new(Vec::new()));
    let latest_values_cloned = latest_values.clone();
    let window_count = Shared::new(Mutable::new(0usize));
    let window_count_cloned = window_count.clone();
    let boundary_reentrant = Shared::new(Mutable::new(Some(boundary)));
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let index = window_count_cloned.lock_mut(|mut lock| {
                let index = *lock;
                *lock += 1;
                index
            });
            match index {
                0 => {
                    let first_inner_subscription = first_inner_subscription_outer.clone();
                    let first_inner_subscription_cloned = first_inner_subscription.clone();
                    let first_terminations = first_terminations_cloned.clone();
                    let boundary_reentrant = boundary_reentrant.clone();
                    let sub = window.subscribe_with_callback(
                        move |_value| {
                            // Both rollovers are queued while the old observer is
                            // still handling a value. The middle window is closed
                            // before it can be delivered to the outer observer.
                            let mut boundary =
                                boundary_reentrant.lock_mut(|mut lock| lock.take()).unwrap();
                            boundary.on_next(());
                            boundary.on_next(());
                            let sub =
                                first_inner_subscription_cloned.lock_mut(|mut lock| lock.take());
                            drop(sub);
                        },
                        move |termination| safe_lock_vec!(push: first_terminations, termination),
                    );
                    first_inner_subscription.lock_mut(|mut lock| *lock = Some(sub));
                }
                1 => {
                    let second_terminations = second_terminations_cloned.clone();
                    let sub = window.subscribe_with_callback(
                        |_value| {},
                        move |termination| safe_lock_vec!(push: second_terminations, termination),
                    );
                    safe_lock_vec!(push: inner_subscriptions_cloned, sub);
                }
                2 => {
                    let latest_values = latest_values_cloned.clone();
                    let sub = window.subscribe_with_callback(
                        move |value| safe_lock_vec!(push: latest_values, value),
                        |_termination| {},
                    );
                    safe_lock_vec!(push: inner_subscriptions_cloned, sub);
                }
                _ => unreachable!(),
            }
        },
        |_termination| {},
    );

    source.on_next(111);
    source.on_next(222);

    assert_eq!(safe_lock!(clone: window_count), 3);
    assert_eq!(safe_lock_vec!(len: first_terminations), 0);
    assert_eq!(
        safe_lock!(clone: second_terminations),
        [Termination::Completed]
    );
    assert_eq!(safe_lock!(clone: latest_values), [222]);
}

#[cfg(not(feature = "single-threaded"))]
#[cfg(panic = "unwind")]
#[test]
fn test_dropping_ignored_value_stops_the_subscription() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, channel_checker) = test_channel::<'_, PanicOnDrop, Infallible>();
    let (_boundary_sender, boundary_observable, boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();

    // Custom operations
    let observable = observable.window(boundary_observable);

    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let sub = window.subscribe_with_callback(|_value| {}, |_termination| {});
            safe_lock_vec!(push: inner_subscriptions_cloned, sub);
        },
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: inner_subscriptions), 1);

    // Stop the current inner subscription while keeping the outer window
    // pipeline active, so subsequent source values take the ignored-value path.
    let inner_subscription = inner_subscriptions.lock_mut(|mut lock| lock.pop()).unwrap();
    drop(inner_subscription);

    // The window that the value belongs to is closed, so the value is dropped while the pipeline
    // is delivering it. A panic from that drop stops the subscription, like a panic from any other
    // call made while delivering, which disposes the source and the boundary.
    expect_panic_on_drop(|value| sender.on_next(value));
    // The subscription is over, so no window is emitted anymore.
    assert_eq!(safe_lock_vec!(len: inner_subscriptions), 0);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(boundary_channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe_window_subscription_keeps_stream_working() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, _boundary_channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window(boundary_observable);

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |termination| termination_observer.on_termination(termination),
    );

    // Subscribe to window 1 and then unsubscribe in the middle of the window.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let sub_1 = window_1.subscribe(observer_1);
    sender.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    drop(sub_1);

    // The window holds the observer between two events, so unsubscribing does not release it: the
    // window does, on its next event.
    assert_eq!(checker_1.state(), State::Active);

    // Later values of this window have nowhere to go, but the pipeline stays healthy. The value
    // below is what makes the window notice the disposal and release the observer.
    sender.on_next(222);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);

    // The next window works as usual.
    boundary_sender.on_next(());
    let window_2 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    sender.on_next(333);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(termination_checker.state(), State::Active);
}

#[test]
fn test_dropping_unsubscribed_inner_observable_releases_buffered_values() {
    use rx_rust::utils::types::MutableHelper;

    struct DropTracker(Shared<Mutable<usize>>);

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.0.lock_mut(|mut lock| *lock += 1);
        }
    }

    let (mut sender, observable, _channel_checker) = test_channel::<'_, DropTracker, Infallible>();
    let (_boundary_sender, boundary_observable, _boundary_channel_checker) =
        test_channel::<'_, (), Infallible>();

    // Custom operations
    let observable = observable.window(boundary_observable);

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );
    assert_eq!(safe_lock_vec!(len: window_vec), 1);

    let drop_count = Shared::new(Mutable::new(0usize));
    sender.on_next(DropTracker(drop_count.clone()));
    assert_eq!(safe_lock!(clone: drop_count), 0);

    // Dropping the only handle to an unsubscribed window must release its
    // buffered values even while the outer window subscription remains active.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    drop(window_1);
    assert_eq!(safe_lock!(clone: drop_count), 1);
}

#[test]
fn test_unsubscribed_window_values_do_not_leak_into_next_window() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut boundary_sender, boundary_observable, _boundary_channel_checker) = test_channel();

    // Custom operations
    let observable = observable.window(boundary_observable);

    // Hold the window observables without subscribing to them.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| safe_lock_vec!(push: window_vec_cloned, window),
        |_termination| {},
    );

    // Window 1 is never subscribed; its values buffer up.
    sender.on_next(111);
    sender.on_next(222);

    // The boundary closes window 1. Its undelivered values belong to window 1
    // and must die with it instead of rolling over into window 2.
    boundary_sender.on_next(());
    assert_eq!(safe_lock_vec!(len: window_vec), 2);

    let window_2 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    assert_eq!(checker_2.values(), []);

    // Window 2 receives only its own values.
    sender.on_next(333);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);

    // The stale window 1 stays consistent: completion without values.
    let window_1 = window_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker_1 = TestStruct;
    let life_marker_2 = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let boundary_subject = Create::new(|mut observer| {
            observer.on_next(());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = observable.window(boundary_subject);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
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
        let observable = Create::new(|observer: BoxedObserver<'_, &TestStruct, Infallible>| {
            life_marker_or = Some(observer);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker_sub.consume_ref();
            }))
        });
        let boundary_subject = Create::new(|_| Subscription::default());
        let observable = observable.window(boundary_subject);

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
    let boundary_subject = Create::new(|_| Subscription::default());
    let observable = observable.window(boundary_subject);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject = PublishSubject::default();
    let observable = subject.window(boundary_subject);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.window(boundary_subject);

    observable.filter(|_| true);
}
