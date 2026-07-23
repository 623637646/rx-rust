mod tests_utils;

use rx_rust::utils::types::{MutableBool, MutableBoolHelper};
use rx_rust::{
    observable::{Observable, ObservableExt, Subscription},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::creating::create::Create,
    safe_lock, safe_lock_option,
    utils::{
        subscribe_with_context::{ContextStopped, EventBatch, ModelUpdate, subscribe_with_context},
        types::{Mutable, Shared},
    },
};
use std::convert::Infallible;
use tests_utils::{
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
};

#[test]
fn delivers_batch_in_order_with_termination() {
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let mut context_out = None;
    let _subscription = subscribe_with_context(observer, (), |context| {
        context_out = Some(context);
        Subscription::default()
    });
    let context = context_out.unwrap();

    context.send_events(EventBatch::NextBatchAndTermination(
        vec![1, 2, 3],
        Termination::Completed,
    ));
    assert_eq!(checker.values(), [1, 2, 3]);
    assert_eq!(checker.state(), State::Completed);
}

// Dispose from inside `on_next` of a middle-of-batch value: the remaining
// queued values must not be delivered, and the observer must be dropped
// without a termination.
#[test]
fn dispose_during_batch_stops_remaining_events() {
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let subscription_slot = Shared::new(Mutable::new(None));
    let dispose_slot = subscription_slot.clone();
    let mut context_out = None;
    let subscription = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        subscribe_with_context(observer, (), |context| {
            context_out = Some(context);
            Subscription::default()
        })
    })
    .hook_on_next(move |observer, value| {
        observer.on_next(value);
        if value == 2 {
            // Dropping the subscription disposes it.
            drop(safe_lock_option!(take: dispose_slot));
        }
    })
    .subscribe(observer);
    safe_lock_option!(replace: subscription_slot, subscription);
    let context = context_out.unwrap();

    context.send_events(EventBatch::NextBatch(vec![1, 2, 3]));
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);

    // Later events are ignored as well.
    context.send_next(4);
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn dispose_during_batch_suppresses_pending_termination() {
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let subscription_slot = Shared::new(Mutable::new(None));
    let dispose_slot = subscription_slot.clone();
    let mut context_out = None;
    let subscription = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        subscribe_with_context(observer, (), |context| {
            context_out = Some(context);
            Subscription::default()
        })
    })
    .hook_on_next(move |observer, value| {
        observer.on_next(value);
        if value == 2 {
            drop(safe_lock_option!(take: dispose_slot));
        }
    })
    .subscribe(observer);
    safe_lock_option!(replace: subscription_slot, subscription);
    let context = context_out.unwrap();

    context.send_events(EventBatch::NextBatchAndTermination(
        vec![1, 2],
        Termination::Completed,
    ));
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn reentrant_send_is_queued_after_pending_events() {
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let context_slot = Shared::new(Mutable::new(None));
    let send_slot = context_slot.clone();
    let mut context_out = None;
    let subscription = Create::new(|observer: BoxedObserver<'_, i32, Infallible>| {
        subscribe_with_context(observer, (), |context| {
            safe_lock_option!(replace: context_slot, context.clone());
            context_out = Some(context);
            Subscription::default()
        })
    })
    .hook_on_next(move |observer, value| {
        observer.on_next(value);
        if value == 1 {
            let context = safe_lock!(clone: send_slot).unwrap();
            context.send_next(10);
        }
    })
    .subscribe(observer);
    let context = context_out.unwrap();

    context.send_events(EventBatch::NextBatch(vec![1, 2, 3]));
    assert_eq!(checker.values(), [1, 2, 3, 10]);
    assert_eq!(checker.state(), State::Active);

    drop(subscription);
    assert_eq!(checker.state(), State::Dropped);
    context.send_next(4);
    assert_eq!(checker.values(), [1, 2, 3, 10]);
}

#[test]
fn empty_batch_is_a_no_op() {
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let mut context_out = None;
    let _subscription = subscribe_with_context(observer, (), |context| {
        context_out = Some(context);
        Subscription::default()
    });
    let context = context_out.unwrap();

    context.send_events(EventBatch::NextBatch(vec![]));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);

    context.send_next(1);
    assert_eq!(checker.values(), [1]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn stopped_try_update_model_drops_callback_outside_lock() {
    struct RunOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for RunOnDrop<F> {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    let (checker, observer) = Checker::<i32, Infallible>::new();
    let mut context_out = None;
    let subscription = subscribe_with_context(observer, (), |context| {
        context_out = Some(context);
        Subscription::default()
    });
    let context = context_out.unwrap();
    drop(subscription);
    assert_eq!(checker.state(), State::Dropped);

    let callback_dropped = Shared::new(MutableBool::new(false));
    let callback_dropped_on_drop = callback_dropped.clone();
    let reentrant_context = context.clone();
    let run_on_drop = RunOnDrop(Some(move || {
        callback_dropped_on_drop.write(true);
        // This would deadlock if the callback were dropped while the context was locked.
        reentrant_context.send_next(1);
    }));

    let result = context.try_update_model(move |_| {
        drop(run_on_drop);
        ModelUpdate::new_empty()
    });

    assert_eq!(result, Err(ContextStopped));
    assert!(callback_dropped.read());
    assert_eq!(checker.values(), []);
}

// End-to-end wiring through a real upstream: the channel feeds the context the
// way real operators do (forwarding via a `WeakSubscriptionContext` to avoid a strong
// reference cycle). Disposing mid-batch must also unsubscribe the upstream.
#[test]
fn dispose_during_batch_unsubscribes_upstream() {
    let (mut sender, receiver, channel_checker) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let subscription_slot = Shared::new(Mutable::new(None));
    let dispose_slot = subscription_slot.clone();
    let context_slot = Shared::new(Mutable::new(None));
    let send_slot = context_slot.clone();

    let subscription = receiver
        .hook_on_subscription(|source, observer: BoxedObserver<'_, i32, Infallible>| {
            subscribe_with_context(observer, (), |context| {
                safe_lock_option!(replace: context_slot, context.clone());
                let weak = context.downgrade();
                let weak_for_termination = weak.clone();
                source.subscribe_with_callback(
                    move |value| {
                        if let Some(context) = weak.upgrade() {
                            context.send_next(value);
                        }
                    },
                    move |termination| {
                        if let Some(context) = weak_for_termination.upgrade() {
                            context.send_termination(termination);
                        }
                    },
                )
            })
        })
        .hook_on_next(move |observer, value| {
            observer.on_next(value);
            if value == 1 {
                // Queue a batch while value 1 is still being processed.
                let context = safe_lock!(clone: send_slot).unwrap();
                context.send_events(EventBatch::NextBatch(vec![2, 3]));
            }
            if value == 2 {
                drop(safe_lock_option!(take: dispose_slot));
            }
        })
        .subscribe(observer);
    safe_lock_option!(replace: subscription_slot, subscription);

    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    sender.on_next(1);
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}
