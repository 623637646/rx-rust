mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::create::Create;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::others::hook_on_termination::HookOnTermination,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = observable.hook_on_termination(move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Error("error"));
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_completed_no_call_original() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.hook_on_termination(move |_, termination| {
        observer_2.on_termination(termination);
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_completed_send_values_before_terminated() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_termination(move |mut observer, termination| {
        assert!(observer.on_next(222).is_continue());
        observer.on_termination(termination);
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.hook_on_termination(move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Completed);
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let terminations = Arc::new(Mutex::new(Vec::new()));
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = observable.hook_on_termination(|observer, termination| {
        match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                assert_eq!(error, "error");
                observer.on_termination(Termination::Error("hooked"));
            }
        }
        terminations.with_mut(|values| values.push(termination));
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [111, 222]);
    assert_eq!(checker_2.state(), State::Error("hooked"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
    assert_eq!(
        terminations.clone_value(),
        vec![Termination::Error("error")]
    );
}

#[test]
fn test_ref() {
    let value = 111;
    let error_1 = 222;
    let error_2 = 333;

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    let (mut sender, observable, channel_checker) = test_channel();

    // Custom operations
    let observable = observable.hook_on_termination(|observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Error(&error_2));
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(sender.on_next(&value).is_continue());
    assert_eq!(checker_1.values(), [&value]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    sender.on_termination(Termination::Error(&error_1));
    assert_eq!(checker_1.values(), [&value]);
    assert_eq!(checker_1.state(), State::Error(&error_2));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error_1));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error(&error_1));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error_1 = 222;
    let mut error_2 = 333;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error_1));
        Subscription::default()
    });
    let (checker, observer) = Checker::<Infallible, _>::new();
    let (_, on_termination) = observer.into_callbacks();

    // Custom operations
    let observable = observable.hook_on_termination(|observer, mut termination| {
        match &mut termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(**error));
                **error *= 2;
            }
        }
        observer.on_termination(Termination::Error(&mut error_2));
    });

    let _subscription = observable.subscribe_with_callback(
        |value| {
            *value *= 2;
        },
        |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                *error *= 2;
            }
        },
    );

    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(value, 222);
    assert_eq!(error_1, 444);
    assert_eq!(error_2, 666);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = observable.hook_on_termination(move |observer, termination| {
            observer_2.on_termination(termination);
            observer.on_termination(Termination::Completed);
            panic!()
        });

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer_1) })
            .await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let terminations = Arc::new(Mutex::new(Vec::new()));

    // Custom operations
    let terminations_cloned = terminations.clone();
    let observable = observable.hook_on_termination_boxed(move |observer, termination| {
        terminations_cloned.with_mut(|values| values.push(termination));
        observer.on_termination(Termination::Completed);
    });
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert!(terminations.with_ref(Vec::is_empty));

    channels.on_termination(0, Termination::Error("error"));
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Error("error"));
    assert_eq!(
        terminations.clone_value(),
        vec![Termination::Error("error"), Termination::Error("error")]
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<(), _>::new();

    // Custom operations
    let observable = observable
        .hook_on_termination(move |observer, termination| {
            observer_2.on_termination(termination);
            observer.on_termination(Termination::Error("error"));
        })
        .take(1);

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();
    let (checker_3, observer_3) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable
        .hook_on_termination(move |observer, termination| {
            observer_2.on_termination(termination);
            observer.on_termination(Termination::Error("222"));
        })
        .hook_on_termination(move |observer, termination| {
            observer_3.on_termination(termination);
            observer.on_termination(Termination::Error("333"));
        });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    sender.on_termination(Termination::Error("111"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("333"));
    assert_eq!(channel_checker.state(), ChannelState::Error("111"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error("111"));
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Error("222"));
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = HookOnTermination::new(observable, move |observer, termination| {
        observer_2.on_termination(termination);
        observer.on_termination(Termination::Completed);
    });

    let _subscription = observable.subscribe(observer_1);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Error("error"));
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
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.hook_on_termination(|_, _| {});

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
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.hook_on_termination(|_, _| {});

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let s = TestStruct;

    let (_, observable, _) = test_channel::<'_, i32, &str>();

    // Custom operations
    let observable = observable.hook_on_termination(|_, _| {
        s.consume();
    });

    let _ = observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.hook_on_termination_boxed(|_, _| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.hook_on_termination(|_, _| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.hook_on_termination_boxed(|_, _| {});

    observable.filter(|_| true);
}
