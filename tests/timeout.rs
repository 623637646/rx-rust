mod tests_utils;

use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::{
    DURATION_3_MS, DURATION_10_MS, DURATION_30_MS, DURATION_100_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::{
    disposable::Disposable,
    disposable::callback_disposal::CallbackDisposal,
    observable::Subscription,
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{empty::Empty, never::Never, throw::Throw},
        utility::timeout::{self, Timeout},
    },
};
use std::{convert::Infallible, time::Duration};

#[test]
fn test_completed() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_timeout() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_timeout_0_duration() {
    block_on(|scheduler| async move {
        let (_, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(Duration::ZERO, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

        scheduler.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

/// `Never` drops its observer as soon as it is subscribed: a source that will never send anything
/// times out like one that merely falls silent. See decision 0004.
#[test]
fn test_timeout_never() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        let observable = Never.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_30_MS * 2).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
    });
}
#[test]
fn test_unsubscribe() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        subscription.dispose();
        // The timer task still holds the context, but the source dropped its own handle as it was
        // disposed, which released the observer.
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|scheduler| async move {
        let (channels, observable) = test_channels::<'_, _, Infallible>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        assert!(channels.on_next(0, 111).is_continue());
        assert!(channels.on_next(1, 111).is_continue());
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(channels.on_next(0, 222).is_continue());
        assert!(channels.on_next(1, 222).is_continue());
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);
        assert_eq!(channels.state(1), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable
            .timeout(DURATION_100_MS, scheduler.clone())
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_stop());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation_timeout_at_first() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable
            .timeout(DURATION_100_MS, scheduler.clone())
            .timeout(DURATION_100_MS * 2, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError(timeout::Error::Timeout))
        );
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation_timeout_at_sencond() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable
            .timeout(DURATION_100_MS * 2, scheduler.clone())
            .timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}
#[test]
fn test_without_convenient_api() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = Timeout::new(observable, DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert!(sender.on_next(222).is_continue());
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

/// The source goes away without a termination: it has fallen silent for good, which is what a
/// timeout is there to report, so the timer still fires and fails the stream.
/// See decision 0004: a source that drops its observer without a termination only stops sending;
/// what the operator has already accepted runs its course.
#[test]
fn test_abandon_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(sender.on_next(111).is_continue());
        sender.abandon();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);

        scheduler.sleep(DURATION_100_MS - DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);

        scheduler.sleep(DURATION_30_MS * 2).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    });
}

/// Disposing after the source went away still cancels the timer: no timeout is reported.
#[test]
fn test_unsub_after_abandon() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(sender.on_next(111).is_continue());
        sender.abandon();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Abandoned);
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(sender.on_next(*i).is_continue());
        }
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|scheduler| async move {
        let (sender, source, _) = test_channel();
        let source = source.start_with([111]);
        let (checker, observer) = Checker::new();

        let observable = source.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_next_on_sub_with_timeout() {
    block_on(|scheduler| async move {
        let (_, source, _) = test_channel::<'_, i32, Infallible>();
        let source = source.start_with([111]);
        let (checker, observer) = Checker::new();

        let observable = source.timeout(Duration::ZERO, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        let observable = Empty.timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_on_sub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        let observable = Throw::new("error").timeout(DURATION_100_MS, scheduler.clone());

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), []);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source emits from inside its own disposal, so the value arrives while downstream is
        // unsubscribing. It must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_stop());
                }))
            });

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source completes from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The termination must be dropped instead of reaching the observer.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                Subscription::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_error_on_unsub() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();

        // The source fails from inside its own disposal, so it terminates while downstream is
        // unsubscribing. The error must be dropped instead of reaching the observer.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        });

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_clone() {
    block_on(|scheduler| async move {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(TestStruct).is_continue());
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|scheduler| async move {
        let (_, observable, _) = test_channel::<'_, i32, Infallible>();

        let observable = observable.timeout(DURATION_100_MS, scheduler.clone());
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|scheduler| async move {
        let (_, observable, _) = test_channel::<'_, i32, Infallible>();

        observable.timeout(DURATION_100_MS, scheduler.clone());
    });
}
