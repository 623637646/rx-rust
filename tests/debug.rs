mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::operators::creating::create::Create;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::others::debug::{Debug, DebugEvent},
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_struct::TestStruct};

/// An owned copy of a [`DebugEvent`], which borrows the value it reports.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Recorded<T, E> {
    Subscribed,
    Next(T),
    Termination(Termination<E>),
    Disposed,
}

/// The events the operator reported, in order. It is the operator's context, so the callback is a
/// plain `fn`, as `Clone` as the operator needs it to be.
type Log<T, E> = Arc<Mutex<Vec<Recorded<T, E>>>>;

fn new_log<T, E>() -> Log<T, E> {
    Arc::new(Mutex::new(Vec::new()))
}

fn record<T: Clone, E: Clone>(log: Log<T, E>, event: DebugEvent<'_, T, E>) {
    let recorded = match event {
        DebugEvent::Subscribed => Recorded::Subscribed,
        DebugEvent::OnNext(value) => Recorded::Next(value.clone()),
        DebugEvent::OnTermination(termination) => Recorded::Termination(termination.clone()),
        DebugEvent::Disposed => Recorded::Disposed,
    };
    log.with_mut(|log| log.push(recorded));
}

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let log = new_log();

    let observable = observable.debug(log.clone(), record);

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(log.clone_value(), [Recorded::Subscribed]);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(
        log.clone_value(),
        [Recorded::Subscribed, Recorded::Next(111)]
    );

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Completed),
        ]
    );

    // A terminated subscription still reports its disposal.
    drop(subscription);
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Completed),
            Recorded::Disposed,
        ]
    );
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let log = new_log();

    let observable = observable.debug(log.clone(), record);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Error("error")),
        ]
    );
}

/// The callback runs before what it reports: before the source is subscribed to, before a value
/// or the termination is forwarded, and only once the source has been disposed.
#[test]
fn test_order() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let checker_cloned = checker.clone();
    let channel_checker_cloned = channel_checker.clone();
    let observable = observable.debug((), move |(), event| match event {
        DebugEvent::Subscribed => {
            assert_eq!(channel_checker_cloned.state(), ChannelState::Initialized);
        }
        DebugEvent::OnNext(_) => assert!(checker_cloned.values().is_empty()),
        DebugEvent::OnTermination(_) => assert_eq!(checker_cloned.state(), State::Active),
        DebugEvent::Disposed => {
            assert_eq!(channel_checker_cloned.state(), ChannelState::Unsubscribed);
        }
    });

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let log = new_log();

    let observable_1 = observable.debug(log.clone(), record);
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [222]);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Next(222),
            Recorded::Disposed,
            Recorded::Termination(Termination::Error("error")),
        ]
    );
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let log = new_log();

    let observable = observable.debug(log.clone(), record);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(&value).is_continue());
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(&value),
            Recorded::Termination(Termination::Error(&error)),
        ]
    );
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;
    let log = new_log::<i32, i32>();

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });

    // The callback sees a borrow of the item, so it reads through the `&mut` without taking it.
    let observable = observable.debug(log.clone(), |log, event| {
        let recorded = match event {
            DebugEvent::Subscribed => Recorded::Subscribed,
            DebugEvent::OnNext(value) => Recorded::Next(**value),
            DebugEvent::OnTermination(Termination::Completed) => {
                Recorded::Termination(Termination::Completed)
            }
            DebugEvent::OnTermination(Termination::Error(error)) => {
                Recorded::Termination(Termination::Error(**error))
            }
            DebugEvent::Disposed => Recorded::Disposed,
        };
        log.with_mut(|log| log.push(recorded));
    });

    let subscription = observable.subscribe_with_callback(
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

    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Error(222)),
        ]
    );
    drop(subscription);
    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
        let (checker, observer) = Checker::new();
        let log = new_log();

        let observable = observable.debug(log.clone(), record);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(
            log.clone_value(),
            [
                Recorded::Subscribed,
                Recorded::Next(111),
                Recorded::Disposed
            ]
        );
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let log = new_log();

    let observable_1 = observable.debug(log.clone(), record);
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [111]);

    channels.on_termination(0, Termination::Error("error"));
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Next(111),
            Recorded::Termination(Termination::Error("error")),
            Recorded::Termination(Termination::Error("error")),
        ]
    );
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let log = new_log();

    let observable = observable.debug(log.clone(), record).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    // `take` disposes the subscription it holds, which is the one of this operator.
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Disposed
        ]
    );
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();
    let log_1 = new_log();
    let log_2 = new_log();

    let observable = observable
        .debug(log_1.clone(), record)
        .map(|value| value * 2)
        .debug(log_2.clone(), record);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [222]);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        log_1.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Error("error")),
        ]
    );
    assert_eq!(
        log_2.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(222),
            Recorded::Termination(Termination::Error("error")),
        ]
    );
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::new();
    let log = new_log();

    let observable = Debug::new(observable, log.clone(), record);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(
        log.clone_value(),
        [
            Recorded::Subscribed,
            Recorded::Next(111),
            Recorded::Termination(Termination::Error("error")),
        ]
    );
}

/// The printing form forwards everything unchanged; what it prints is not checked.
#[test]
fn test_debug_to_stdout() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    let observable = observable.debug_to_stdout("label");

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.debug((), |(), _| {});

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
            DisposeOnDrop::default()
        });
        let observable = observable.debug((), |(), _| {});

        let (_, mut observer) = Checker::<_, Infallible>::new();
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
    let observable = observable.debug((), |(), _| {});
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.debug((), |(), _| {});

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let observable = observable.debug((), |(), _| {});

    observable.filter(|_| true);
}
