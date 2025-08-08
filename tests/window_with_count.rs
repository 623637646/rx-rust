mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::window_with_count::WindowWithCount},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
};
use rx_rust::{safe_lock, safe_lock_vec};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

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

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
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

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

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

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
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

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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

    sender.on_termination(Termination::Error("error"));
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

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

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
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

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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

    subscription.dispose();
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, sub)) in safe_lock!(mem_take: checker_sub_vec)
        .into_iter()
        .enumerate()
    {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                sub.dispose();
                assert_eq!(checker.state(), State::Dropped);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

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

    sender.on_next(&value_2);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1, &value_2]);
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

    sender.on_next(&value_3);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1, &value_2]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_3]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [&value_1, &value_2]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [&value_3]);
                assert_eq!(checker.state(), State::Error(&error));
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
        let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

        // Custom operations
        let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

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
                    assert_eq!(checker.values(), [111, 222]);
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
                    assert_eq!(checker.values(), [111, 222]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [333]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move { sender.on_termination(Termination::Completed) })
            .await
            .unwrap();
        assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
        for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111, 222]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), [333]);
                    assert_eq!(checker.state(), State::Completed);
                }
                _ => panic!(),
            }
        }
        assert_eq!(termination_checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
    let (termination_checker_1, termination_observer_1) = Checker::<Infallible, _>::new();
    let checker_sub_vec_1 = Shared::new(Mutable::new(Vec::new()));
    let (termination_checker_2, termination_observer_2) = Checker::<Infallible, _>::new();
    let checker_sub_vec_2 = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window_with_count(NonZeroUsize::new(1).unwrap());
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
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 3);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
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
                assert_eq!(checker.values(), [222]);
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

    subject.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 4);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Completed);
            }
            3 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 4);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Completed);
            }
            3 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_1), 4);
    for (index, (checker, _)) in checker_sub_vec_1.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Completed);
            }
            3 => {
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker_1.state(), State::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec_2), 4);
    for (index, (checker, _)) in checker_sub_vec_2.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Completed);
            }
            2 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Completed);
            }
            3 => {
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
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable
        .window_with_count(NonZeroUsize::new(2).unwrap())
        .take(1);

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
    assert_eq!(termination_checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let context = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = observable
        .window_with_count(NonZeroUsize::new(1).unwrap())
        .window_with_count(NonZeroUsize::new(2).unwrap());

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

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: context), 3);
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
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
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
            2 => {
                assert_eq!(safe_lock_vec!(len: checker), 0);
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
                assert_eq!(safe_lock_vec!(len: checker), 2);
                for (index, (checker, _)) in checker.test_lock_ref().iter().enumerate() {
                    match index {
                        0 => {
                            assert_eq!(checker.values(), [333]);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        1 => {
                            assert_eq!(checker.values(), []);
                            assert_eq!(checker.state(), State::Completed);
                        }
                        _ => panic!(),
                    }
                }
            }
            2 => {
                assert_eq!(safe_lock_vec!(len: checker), 0);
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
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = WindowWithCount::new(observable, NonZeroUsize::new(2).unwrap());

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

    sender.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
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

    sender.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(444);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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

    sender.on_termination(Termination::Completed);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 3);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 222]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [333, 444]);
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
}

#[test]
fn test_revert_completed() {
    let mut subject = PublishSubject::<'_, _, Infallible>::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());
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
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());
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
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

        let (_, observer) = Checker::<_, ()>::new();
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
        let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

        let (_, mut observer) = Checker::<_, Infallible>::new();
        let mut subject = PublishSubject::default();
        subject.on_next(&life_marker_2);
        let subject_observable = SubjectObservable::new(subject);
        observer.on_next(subject_observable);
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
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.window_with_count(NonZeroUsize::new(2).unwrap());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.window_with_count(NonZeroUsize::new(2).unwrap());

    observable.filter(|_| true);
}
