mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::mutable::Mutable;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::utils::types::Shared;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, transforming::window_with_count::WindowWithCount},
    subject::{publish_subject::PublishSubject, unicast_subject::unicast_subject},
};
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    for (index, (checker, sub)) in checker_sub_vec.take_value().into_iter().enumerate() {
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
                // Disposing the outer subscription drops the sending end of the open window, so
                // its observer is dropped too: nothing can reach it anymore.
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Dropped);
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert!(checker.values().is_empty());
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_1).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [&value_1]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(&value_3).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
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
                        checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
                    },
                    |termination| {
                        termination_observer.on_termination(termination);
                    },
                )
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
        checker_sub_vec.with_ref(|checker_sub_vec| {
            for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
                match index {
                    0 => {
                        assert_eq!(checker.values(), []);
                        assert_eq!(checker.state(), State::Active);
                    }
                    _ => panic!(),
                }
            }
        });
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
        checker_sub_vec.with_ref(|checker_sub_vec| {
            for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
                match index {
                    0 => {
                        assert_eq!(checker.values(), [111]);
                        assert_eq!(checker.state(), State::Active);
                    }
                    _ => panic!(),
                }
            }
        });
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
        checker_sub_vec.with_ref(|checker_sub_vec| {
            for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
        });
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let sender = runtime
            .spawn(async move {
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
        checker_sub_vec.with_ref(|checker_sub_vec| {
            for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
        });
        assert_eq!(termination_checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move { sender.on_termination(Termination::Completed) })
            .await
            .unwrap();
        assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
        checker_sub_vec.with_ref(|checker_sub_vec| {
            for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
        });
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer_2.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec_1.with_ref(Vec::len), 1);
    checker_sub_vec_1.with_ref(|checker_sub_vec_1| {
        for (index, (checker, _)) in checker_sub_vec_1.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(checker_sub_vec_2.with_ref(Vec::len), 1);
    checker_sub_vec_2.with_ref(|checker_sub_vec_2| {
        for (index, (checker, _)) in checker_sub_vec_2.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker_2.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_sub_vec_1.with_ref(Vec::len), 2);
    checker_sub_vec_1.with_ref(|checker_sub_vec_1| {
        for (index, (checker, _)) in checker_sub_vec_1.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(checker_sub_vec_2.with_ref(Vec::len), 2);
    checker_sub_vec_2.with_ref(|checker_sub_vec_2| {
        for (index, (checker, _)) in checker_sub_vec_2.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_2.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_sub_vec_1.with_ref(Vec::len), 3);
    checker_sub_vec_1.with_ref(|checker_sub_vec_1| {
        for (index, (checker, _)) in checker_sub_vec_1.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(checker_sub_vec_2.with_ref(Vec::len), 3);
    checker_sub_vec_2.with_ref(|checker_sub_vec_2| {
        for (index, (checker, _)) in checker_sub_vec_2.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_2.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker_sub_vec_1.with_ref(Vec::len), 4);
    checker_sub_vec_1.with_ref(|checker_sub_vec_1| {
        for (index, (checker, _)) in checker_sub_vec_1.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_1.state(), State::Active);
    assert_eq!(checker_sub_vec_2.with_ref(Vec::len), 4);
    checker_sub_vec_2.with_ref(|checker_sub_vec_2| {
        for (index, (checker, _)) in checker_sub_vec_2.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_2.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec_1.with_ref(Vec::len), 4);
    checker_sub_vec_1.with_ref(|checker_sub_vec_1| {
        for (index, (checker, _)) in checker_sub_vec_1.iter().enumerate() {
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
    });
    assert_eq!(termination_checker_1.state(), State::Completed);
    assert_eq!(checker_sub_vec_2.with_ref(Vec::len), 4);
    checker_sub_vec_2.with_ref(|checker_sub_vec_2| {
        for (index, (checker, _)) in checker_sub_vec_2.iter().enumerate() {
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
    });
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    // Taking one window disposes the outer subscription, which drops the sending end
                    // of the open window, so its observer is dropped too.
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Dropped);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Completed);
    // The first window is emitted before the source is subscribed to, so taking one window ends
    // the stream while there is nothing to subscribe to yet: the source is never reached.
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
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
                    checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
                },
                |_| {},
            );
            context_cloned.with_mut(|values| values.push((checker_sub_vec, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(context.with_ref(Vec::len), 1);
    context.with_ref(|context| {
        for (index, (checker, _)) in context.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.with_ref(Vec::len), 1);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
                            match index {
                                0 => {
                                    assert_eq!(checker.values(), []);
                                    assert_eq!(checker.state(), State::Active);
                                }
                                _ => panic!(),
                            }
                        }
                    });
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(context.with_ref(Vec::len), 2);
    context.with_ref(|context| {
        for (index, (checker, _)) in context.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                1 => {
                    assert_eq!(checker.with_ref(Vec::len), 0);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(context.with_ref(Vec::len), 2);
    context.with_ref(|context| {
        for (index, (checker, _)) in context.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                1 => {
                    assert_eq!(checker.with_ref(Vec::len), 1);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
                            match index {
                                0 => {
                                    assert_eq!(checker.values(), []);
                                    assert_eq!(checker.state(), State::Active);
                                }
                                _ => panic!(),
                            }
                        }
                    });
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(context.with_ref(Vec::len), 3);
    context.with_ref(|context| {
        for (index, (checker, _)) in context.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                1 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                2 => {
                    assert_eq!(checker.with_ref(Vec::len), 0);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(context.with_ref(Vec::len), 3);
    context.with_ref(|context| {
        for (index, (checker, _)) in context.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                1 => {
                    assert_eq!(checker.with_ref(Vec::len), 2);
                    checker.with_ref(|checker| {
                        for (index, (checker, _)) in checker.iter().enumerate() {
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
                    });
                }
                2 => {
                    assert_eq!(checker.with_ref(Vec::len), 0);
                }
                _ => panic!(),
            }
        }
    });
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
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 3);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
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

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
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

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
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
fn test_subscribe_window_after_values() {
    let mut subject = PublishSubject::<'_, i32, Infallible>::default();
    let windows = Shared::new(Mutable::new(Vec::new()));
    let windows_cloned = windows.clone();
    let _subscription = subject
        .clone()
        .window_with_count(NonZeroUsize::new(2).unwrap())
        .subscribe_with_callback(
            move |window| windows_cloned.with_mut(|values| values.push(window)),
            |_| {},
        );

    assert!(subject.on_next(111).is_continue());
    assert!(subject.on_next(222).is_continue());
    assert!(subject.on_next(333).is_continue());

    // The windows were collected without being subscribed to, so their items are buffered instead
    // of being dropped.
    let mut windows = windows.take_value().into_iter();
    let (checker_1, observer_1) = Checker::new();
    let _subscription_1 = windows.next().unwrap().subscribe(observer_1);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);

    let (checker_2, observer_2) = Checker::new();
    let _subscription_2 = windows.next().unwrap().subscribe(observer_2);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);
    assert!(windows.next().is_none());

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_next_on_sub() {
    let mut subject = BehaviorSubject::new(111);
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject
        .clone()
        .window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    // The window is emitted before the source is subscribed, so the value the
    // subject replays on subscription lands in the first window.
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111]);
                    assert_eq!(checker.state(), State::Active);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
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
    });
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 2);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), [111, 222]);
                    assert_eq!(checker.state(), State::Completed);
                }
                1 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Completed);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Empty.window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Completed);
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Throw::new("error").window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    checker_sub_vec.with_ref(|checker_sub_vec| {
        for (index, (checker, _)) in checker_sub_vec.iter().enumerate() {
            match index {
                0 => {
                    assert_eq!(checker.values(), []);
                    assert_eq!(checker.state(), State::Error("error"));
                }
                _ => panic!(),
            }
        }
    });
    assert_eq!(termination_checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. WindowWithCount hands the observer to its source, so the value still reaches
    // the open window.
    let observable = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_continue());
        }))
    });

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    assert_eq!(termination_checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    for (checker, _sub) in checker_sub_vec.take_value() {
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    }
    assert_eq!(termination_checker.state(), State::Dropped);
}

#[test]
fn test_complete_on_unsub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. WindowWithCount hands the observer to its source, so the completion still
    // reaches the open window and the observer.
    let observable = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    assert_eq!(termination_checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    for (checker, _sub) in checker_sub_vec.take_value() {
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    }
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_error_on_unsub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. WindowWithCount hands the observer to its source, so the error still reaches
    // the open window and the observer.
    let observable = Create::new(|observer: BoxedObserver<'_, i32, &str>| {
        Subscription::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |value| {
            let (checker, observer) = Checker::new();
            let sub = value.subscribe(observer);
            checker_sub_vec_cloned.with_mut(|values| values.push((checker, sub)));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    assert_eq!(termination_checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker_sub_vec.with_ref(Vec::len), 1);
    for (checker, _sub) in checker_sub_vec.take_value() {
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
    }
    assert_eq!(termination_checker.state(), State::Error("error"));
}

#[test]
fn test_subscribe_stale_window_observable() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    // Collect the window observables without subscribing to them immediately.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |_termination| {},
    );
    assert_eq!(window_vec.with_ref(Vec::len), 1);

    // Window 1 is still unsubscribed: its values are buffered. The second value
    // closes it and opens window 2.
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(window_vec.with_ref(Vec::len), 2);

    let mut windows = window_vec.take_value();
    let window_2 = windows.pop().unwrap();
    let window_1 = windows.pop().unwrap();

    // The current window (window 2) receives source values.
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_2.values(), [333]);
    assert_eq!(checker_2.state(), State::Active);

    // Window 1 already completed when it reached the count, so its late
    // subscriber observes the buffered values followed by the completion.
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Completed);

    // Source values keep flowing to the current window's subscriber only.
    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_2.values(), [333, 444]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(window_vec.with_ref(Vec::len), 1);
}

#[test]
fn test_subscribe_current_window_late_with_earlier_values() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(3).unwrap());

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |_termination| {},
    );

    // The value arrives while the current window has no subscriber yet.
    assert!(sender.on_next(111).is_continue());

    // Subscribing replays the buffered value.
    let window_1 = window_vec.with_mut(Vec::pop).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);

    // Later values are delivered directly.
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_1.state(), State::Active);

    // The count includes the buffered value, so the third value closes the window.
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(window_vec.with_ref(Vec::len), 1);
}

#[test]
fn test_subscribe_window_observable_after_termination() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |termination| termination_observer.on_termination(termination),
    );
    assert_eq!(window_vec.with_ref(Vec::len), 1);

    assert!(sender.on_next(111).is_continue());

    // The source terminates while window 1 is still unsubscribed. The window
    // itself was completed by the source termination.
    sender.on_termination(Termination::Completed);
    assert_eq!(termination_checker.state(), State::Completed);

    // A late subscriber observes the buffered value and the completion.
    let window_1 = window_vec.with_mut(Vec::pop).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
}

#[test]
fn test_subscribe_window_observable_after_unsubscribe() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    // Hold the window observable without subscribing to it.
    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |_termination| {},
    );
    assert_eq!(window_vec.with_ref(Vec::len), 1);

    assert!(sender.on_next(111).is_continue());

    // Disposing the outer subscription drops the sender of the open window. Its buffered values
    // are released without completing or erroring the window.
    drop(subscription);

    // A late subscriber receives no buffered values or termination because the window pipe has
    // already been closed.
    let window_1 = window_vec.with_mut(Vec::pop).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let _sub_1 = window_1.subscribe(observer_1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Dropped);
}

#[test]
fn test_unsubscribe_window_subscription_keeps_stream_working() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(3).unwrap());

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |termination| termination_observer.on_termination(termination),
    );

    // Subscribe to window 1 and then unsubscribe in the middle of the window.
    let window_1 = window_vec.with_mut(Vec::pop).unwrap();
    let (checker_1, observer_1) = Checker::new();
    let sub_1 = window_1.subscribe(observer_1);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    drop(sub_1);

    // The window holds the observer between two events, so unsubscribing does not release it: the
    // window does, on its next event.
    assert_eq!(checker_1.state(), State::Active);

    // Later values of this window have nowhere to go, but they still count
    // towards the window size and the pipeline stays healthy. The first of them is what makes the
    // window notice the disposal and release the observer.
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker_1.state(), State::Dropped);
    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [111]);

    // The next window works as usual.
    let window_2 = window_vec.with_mut(Vec::pop).unwrap();
    let (checker_2, observer_2) = Checker::new();
    let _sub_2 = window_2.subscribe(observer_2);
    assert!(sender.on_next(444).is_continue());
    assert_eq!(checker_2.values(), [444]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(termination_checker.state(), State::Active);
}

#[test]
fn test_dropping_unsubscribed_inner_observable_releases_buffered_values() {
    use crate::tests_utils::drop_probe::{DropCount, DropProbe};

    let (mut sender, observable, _channel_checker) = test_channel::<'_, DropProbe, Infallible>();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    let window_vec = Shared::new(Mutable::new(Vec::new()));
    let window_vec_cloned = window_vec.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |window| window_vec_cloned.with_mut(|values| values.push(window)),
        |_termination| {},
    );
    assert_eq!(window_vec.with_ref(Vec::len), 1);

    let drops = DropCount::new();
    assert!(sender.on_next(drops.probe()).is_continue());
    assert_eq!(drops.get(), 0);

    // Dropping the only handle to an unsubscribed window must release its
    // buffered values even while the outer window subscription remains active.
    let window_1 = window_vec.with_mut(Vec::pop).unwrap();
    drop(window_1);
    assert_eq!(drops.get(), 1);
}

#[test]
fn test_dropping_ignored_value_does_not_poison_window_context() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();

    // Custom operations
    let observable = observable.window_with_count(NonZeroUsize::new(2).unwrap());

    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let outer_subscription = observable.subscribe_with_callback(
        move |window| {
            let sub = window.subscribe_with_callback(|_value| {}, |_termination| {});
            inner_subscriptions_cloned.with_mut(|values| values.push(sub));
        },
        |_termination| {},
    );
    assert_eq!(inner_subscriptions.with_ref(Vec::len), 1);

    // Stop the current inner subscription while keeping the outer window
    // pipeline active, so subsequent source values take the ignored-value path.
    let inner_subscription = inner_subscriptions.with_mut(Vec::pop).unwrap();
    drop(inner_subscription);

    expect_panic_on_drop(|value| sender.on_next(value));

    // The ignored value is dropped after releasing the context lock, so the
    // context is still usable after the intentional drop panic: disposing the
    // outer subscription locks the context again and unsubscribes the source.
    drop(outer_subscription);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_non_clone_item() {
    // The windows move their items, so the item type doesn't have to be `Clone`.
    let received = Shared::new(Mutable::new(Vec::new()));
    let received_cloned = received.clone();
    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let source = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        assert!(observer.on_next(TestStruct).is_continue());
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        Subscription::default()
    });
    let _subscription = source
        .window_with_count(NonZeroUsize::new(2).unwrap())
        .subscribe_with_callback(
            move |window| {
                let received = received_cloned.clone();
                let subscription = window.subscribe_with_callback(
                    move |value: TestStruct| {
                        value.consume();
                        received.with_mut(|values| values.push(()));
                    },
                    |_| {},
                );
                inner_subscriptions_cloned.with_mut(|values| values.push(subscription));
            },
            |_| {},
        );
    assert_eq!(received.with_ref(Vec::len), 3);
    inner_subscriptions.take_value().drain(..).for_each(drop);
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
            assert!(observer.on_next(111).is_continue());
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
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
        let (mut sender, window) = unicast_subject();
        assert!(sender.on_next(&life_marker_2).is_continue());
        assert!(observer.on_next(window).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
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
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.window_with_count(NonZeroUsize::new(2).unwrap());

    observable.filter(|_| true);
}
