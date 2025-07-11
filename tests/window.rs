mod tests_utils;

use crate::tests_utils::test_runtime::block_on;
use rx_rust::utils::types::{Mutable, MutableHelper, Shared};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, transforming::window::Window},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
    subscription::{Subscription, disposable::Disposable},
};
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
    assert!(channel_checker.is_completed());
    assert!(boundary_channel_checker.is_unsubscribed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
    assert!(channel_checker.is_unsubscribed());
    assert!(boundary_channel_checker.is_completed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());

    subject.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());

    subject.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), [()]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [()]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), [()]);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_error("error"));
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_error("error"));
    assert!(channel_checker.is_error("error"));
    assert!(boundary_channel_checker.is_unsubscribed());
}

#[test]
fn test_error_from_boundary() {
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_error("error"));
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_error("error"));
    assert!(channel_checker.is_unsubscribed());
    assert!(boundary_channel_checker.is_error("error"));
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    subscription.dispose();
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, sub)) in checker_sub_vec.lock_mut().drain(..).enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
                sub.dispose();
                assert!(checker.is_dropped());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_dropped());
    assert!(channel_checker.is_unsubscribed());
    assert!(boundary_channel_checker.is_unsubscribed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert!(checker.values().is_empty());
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(&value_1);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_completed());
            }
            1 => {
                assert!(checker.values().is_empty());
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(&value_2);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [&value_2]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(&value_3);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert!(checker.is_completed());
            }
            2 => {
                assert!(checker.values().is_empty());
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [&value_2, &value_3]);
                assert!(checker.is_completed());
            }
            2 => {
                assert!(checker.values().is_empty());
                assert!(checker.is_error(&error));
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_error(&error));
    assert!(channel_checker.is_error(&error));
    assert!(boundary_channel_checker.is_unsubscribed());
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
                        checker_sub_vec_cloned.lock_mut().push((checker, sub));
                    },
                    |termination| {
                        termination_observer.on_termination(termination);
                    },
                )
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 1);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 1);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        let mut boundary_sender = runtime
            .spawn(async move {
                boundary_sender.on_next(());
                boundary_sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 2);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_completed());
                }
                1 => {
                    assert_eq!(checker.values(), []);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 2);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_completed());
                }
                1 => {
                    assert_eq!(checker.values(), [222]);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        let sender = runtime
            .spawn(async move {
                sender.on_next(333);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 2);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_completed());
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        let _boundary_sender = runtime
            .spawn(async move {
                boundary_sender.on_next(());
                boundary_sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 3);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_completed());
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert!(checker.is_completed());
                }
                2 => {
                    assert_eq!(checker.values(), []);
                    assert!(checker.is_active());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_active());
        assert!(channel_checker.is_subscribed());
        assert!(boundary_channel_checker.is_subscribed());

        runtime
            .spawn(async move { sender.on_termination(Termination::Completed) })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.lock_ref().len(), 3);
        for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert!(checker.is_completed());
                }
                1 => {
                    assert_eq!(checker.values(), [222, 333]);
                    assert!(checker.is_completed());
                }
                2 => {
                    assert_eq!(checker.values(), []);
                    assert!(checker.is_completed());
                }
                _ => panic!(),
            }
        }
        assert!(termination_checker.is_completed());
        assert!(channel_checker.is_completed());
        assert!(boundary_channel_checker.is_unsubscribed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer_2.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    subject.on_next(111);
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    boundary_subject.on_next(());
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    subject.on_next(222);
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    subject.on_next(333);
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    boundary_subject.on_next(());
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_active());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec_1.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec_1.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_1.is_completed());
    assert_eq!(checker_sub_vec_2.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec_2.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker_2.is_completed());
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
                    checker_sub_vec_cloned.lock_mut().push((checker, sub));
                },
                |_| {},
            );
            context_cloned.lock_mut().push((checker_sub_vec, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(context.lock_ref().len(), 1);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    sender.on_next(111);
    assert_eq!(context.lock_ref().len(), 1);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    boundary_sender_1.on_next(());
    assert_eq!(context.lock_ref().len(), 1);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    boundary_sender_2.on_next(());
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 0);
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    sender.on_next(222);
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 0);
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    boundary_sender_1.on_next(());
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    sender.on_next(333);
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker_1.is_subscribed());
    assert!(boundary_channel_checker_2.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 2);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        1 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
    assert!(channel_checker.is_completed());
    assert!(boundary_channel_checker_1.is_unsubscribed());
    assert!(boundary_channel_checker_2.is_unsubscribed());
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
                    checker_sub_vec_cloned.lock_mut().push((checker, sub));
                },
                |_| {},
            );
            context_cloned.lock_mut().push((checker_sub_vec, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(context.lock_ref().len(), 1);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(context.lock_ref().len(), 1);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    boundary_subject.on_next(());
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(context.lock_ref().len(), 2);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    boundary_subject.on_next(());
    assert_eq!(context.lock_ref().len(), 3);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), []);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(context.lock_ref().len(), 3);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert!(checker.is_active());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(context.lock_ref().len(), 3);
    for (index, (checker, _)) in context.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [111]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            1 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [222]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(checker.lock_ref().len(), 1);
                for (index, (checker, _)) in checker.lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert!(checker.is_completed());
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
    assert!(channel_checker.is_completed());
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
            checker_sub_vec_cloned.lock_mut().push((checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(111);
    assert_eq!(checker_sub_vec.lock_ref().len(), 1);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(222);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_next(333);
    assert_eq!(checker_sub_vec.lock_ref().len(), 2);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    boundary_sender.on_next(());
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_active());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_active());
    assert!(channel_checker.is_subscribed());
    assert!(boundary_channel_checker.is_subscribed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.lock_ref().len(), 3);
    for (index, (checker, _)) in checker_sub_vec.lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert!(checker.is_completed());
            }
            1 => {
                assert_eq!(checker.values(), [222, 333]);
                assert!(checker.is_completed());
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert!(checker.is_completed());
            }
            _ => panic!(),
        }
    }
    assert!(termination_checker.is_completed());
    assert!(channel_checker.is_completed());
    assert!(boundary_channel_checker.is_unsubscribed());
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
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_completed());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_completed());
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
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());
    assert!(checker_3.values().is_empty());
    assert!(checker_3.is_active());

    subject.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    boundary_subject.on_next(());
    assert_eq!(checker_1.values(), [111]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111]);
    assert!(checker_3.is_active());

    subject.on_next(222);
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_active());
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_active());

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111, 222]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [111, 222]);
    assert!(checker_2.is_error("error"));
    assert_eq!(checker_3.values(), [111, 222]);
    assert!(checker_3.is_error("error"));
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
            Subscription::new_with_disposal_callback(|| {
                life_marker_1.consume_ref();
            })
        });
        let boundary_subject = Create::new(|mut observer| {
            observer.on_next(());
            Subscription::new_with_disposal_callback(|| {
                life_marker_2.consume_ref();
            })
        });
        let observable = observable.window(boundary_subject);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_3 = TestStruct;
    let mut life_marker_1 = None;
    let mut life_marker_2 = None;

    // Error
    // let mut life_marker_1 = None;
    // let mut life_marker_2 = None;
    // let life_marker_3 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });
        let boundary_subject = Create::new(|observer| {
            life_marker_2 = Some(observer);
            Subscription::new_none_disposal()
        });
        let observable = observable.window(boundary_subject);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        let mut subject = PublishSubject::default();
        subject.on_next(&life_marker_3);
        let subject_observable = SubjectObservable::new(subject);
        observer.on_next(subject_observable);
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
        let observable = Create::new(|observer: BoxedObserver<'_, &TestStruct, Infallible>| {
            life_marker_or = Some(observer);
            Subscription::new_with_disposal_callback(|| {
                life_marker_sub.consume_ref();
            })
        });
        let boundary_subject = Create::new(|_| Subscription::new_none_disposal());
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
        Subscription::new_none_disposal()
    });
    let boundary_subject = Create::new(|_| Subscription::new_none_disposal());
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
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let boundary_subject: PublishSubject<'_, (), String> = PublishSubject::default();
    let observable = subject.window(boundary_subject);

    observable.filter(|_| true);
}
