mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::observable::{Observable, ObservableExt};
use rx_rust::observer::{Flow, Observer, Termination};
use rx_rust::subject::unicast_subject::{
    UnicastSender, unicast_subject, unicast_subject_with_capacity,
};
use rx_rust::utils::mutable::Mutable;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::utils::types::Shared;
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    drop(subscription);
    assert_eq!(checker.values(), [111]);
    // The sender holds the observer between two events, so disposing cannot drop it: it is the
    // sender that drops it, as soon as it notices, which is the case below.
    assert_eq!(checker.state(), State::Active);
    assert!(sender.is_disposed());

    // The events after the disposal are dropped, and so is the observer.
    assert!(sender.on_next(222).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsubscribe_releases_the_observer_when_the_sender_is_dropped() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.state(), State::Active);

    // Nothing is sent after the disposal, so the observer the sender holds is released by the drop
    // of the sender itself, which is the last chance to do so.
    drop(subscription);
    assert_eq!(checker.state(), State::Active);

    drop(sender);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsubscribe_releases_the_observer_when_the_pipe_terminates() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    drop(subscription);
    assert_eq!(checker.state(), State::Active);

    // The termination reaches no observer, because the observer is gone: it is dropped instead of
    // being notified.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_next_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    // The values sent before the subscription are buffered instead of being dropped.
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(!sender.is_disposed());

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::Completed);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::Error("error"));

    // The buffered values are delivered before the error, unlike a `ReplaySubject`, which drops
    // them.
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_with_capacity() {
    let (mut sender, observable) = unicast_subject_with_capacity::<i32, Infallible>(2);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    // The capacity is only a hint, so the buffer still grows.
    assert!(sender.on_next(333).is_continue());

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_drop_observable_without_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    assert!(sender.on_next(111).is_continue());
    assert!(!sender.is_disposed());

    // Dropping the observable end closes the pipe, so the buffer stops growing.
    drop(observable);
    assert!(sender.is_disposed());

    assert!(sender.on_next(222).is_stop());
    sender.on_termination(Termination::Completed);
}

#[test]
fn test_drop_sender_without_termination() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);

    assert!(sender.on_next(111).is_continue());
    // Nothing can reach the observer once the only sender is gone, so the pipe is closed and the
    // observer is dropped. It is not reported as a completion.
    drop(sender);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_drop_sender_after_termination_keeps_the_buffered_termination() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    assert!(sender.on_next(111).is_continue());
    // Terminating consumes the sender, so this also drops it. The last event still has to reach a
    // late subscriber.
    sender.on_termination(Termination::Completed);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_subscribe_after_drop_sender() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    assert!(sender.on_next(111).is_continue());
    drop(sender);

    // The pipe is closed, so a late subscriber observes neither the buffered values nor a
    // termination.
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let (mut sender, observable) = unicast_subject::<&i32, Infallible>();
    assert!(sender.on_next(&value_1).is_continue());

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (sender, observable) = unicast_subject::<i32, Infallible>();
        let (checker, observer) = Checker::new();

        let mut sender = runtime
            .spawn(async move {
                let mut sender = sender;
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let sender = runtime
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async move {
                sender.on_termination(Termination::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
    });
}

/// Sending a value from an observer that the sender itself is delivering to is impossible: the
/// sender is uniquely owned, so [`Observer::on_next`] holds the only handle to it while the
/// observer runs. The re-entrant path is therefore the replay of the buffered values, which
/// [`Observable::subscribe`] drives while the sender is free.
#[test]
fn test_next_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            let flow = downstream.on_next(value);
            if value == 1 {
                let mut sender = sender_holder_cloned
                    .take_value()
                    .expect("the sender is put back after every re-entrant call");
                assert!(sender.on_next(111).is_continue());
                sender_holder_cloned.replace_value(Some(sender));
            }
            flow
        })
        .subscribe(observer);

    // The re-entrant value comes after the buffered values, in arrival order.
    assert_eq!(checker.values(), [1, 2, 111]);
    assert_eq!(checker.state(), State::Active);

    let mut sender = sender_holder.take_value().unwrap();
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [1, 2, 111, 222]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = observable.subscribe(observer);

    // The observer ended its own stream, so the sender releases it right away and every later
    // value is dropped instead of being delivered.
    assert!(Observer::on_next(&mut sender, 111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(sender.is_disposed());

    assert!(Observer::on_next(&mut sender, 222).is_stop());
    assert_eq!(checker.values(), [111]);
}

#[test]
fn test_stop_on_next_while_replaying() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());

    let (checker, observer) = Checker::stopping_after(1);
    let _subscription = observable.subscribe(observer);

    // The replay stops on the value that ended the stream, so the value still buffered behind it
    // is dropped with the pipe instead of being delivered.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(sender.is_disposed());
}

#[test]
fn test_complete_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            let flow = downstream.on_next(value);
            if value == 111 {
                let sender = sender_holder_cloned.take_value().unwrap();
                sender.on_termination(Termination::Completed);
            }
            flow
        })
        .subscribe(observer);

    // The termination is delivered after the remaining buffered values.
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(sender_holder.with_ref(Option::is_none));
}

#[test]
fn test_error_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            let flow = downstream.on_next(value);
            if value == 111 {
                let sender = sender_holder_cloned.take_value().unwrap();
                sender.on_termination(Termination::Error("error"));
            }
            flow
        })
        .subscribe(observer);

    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(sender_holder.with_ref(Option::is_none));
}

#[test]
fn test_unsub_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = Shared::new(Mutable::new(None));
    let subscription_cloned = subscription.clone();
    subscription.replace_value(Some(
        observable
            .hook_on_next(move |downstream, value| {
                assert!(downstream.on_next(value).is_continue());
                if let Some(subscription) = subscription_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                Flow::Continue
            })
            .subscribe(observer),
    ));

    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(sender.is_disposed());

    assert!(sender.on_next(222).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.take(1).subscribe(observer);

    assert!(sender.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(sender.is_disposed());

    assert!(sender.on_next(222).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_take_during_replay() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender.on_next(333).is_continue());

    // A downstream that stops during the replay of the buffered values closes the pipe once the
    // subscription it disposes exists, which is as soon as `subscribe` returns.
    let (checker, observer) = Checker::new();
    let _subscription = observable.take(1).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(sender.is_disposed());

    assert!(sender.on_next(444).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_on_completed() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = Shared::new(Mutable::new(None));
    let subscription_cloned = subscription.clone();
    subscription.replace_value(Some(
        observable
            .hook_on_termination(move |downstream, termination| {
                if let Some(subscription) = subscription_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                downstream.on_termination(termination);
            })
            .subscribe(observer),
    ));

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

type SenderHolder = Shared<Mutable<Option<UnicastSender<'static, DropProbe, Infallible>>>>;

/// A probe that sends another one into the pipe while being dropped, to check that a value is
/// never dropped while the state of the pipe is locked. Dropping it under the lock would panic in
/// single-threaded builds and deadlock otherwise.
///
/// The probe it sends only counts its own drop, so re-entering never recurses any further.
fn reentrant_probe(holder: &SenderHolder, drops: &DropCount) -> DropProbe {
    let holder = holder.clone();
    let drops = drops.clone();
    DropProbe::new().on_drop(Box::new(move || {
        drops.increment();
        // The holder is emptied while the re-entrant call runs, so the probe sent below re-enters
        // the pipe without recursing any further.
        let Some(mut sender) = holder.take_value() else {
            return;
        };
        // What the pipe answers depends on the test this probe is dropped in; the tests count the
        // drops, and this helper is not where the flow is asserted.
        let _ = sender.on_next(drops.probe());
        holder.replace_value(Some(sender));
    }))
}

#[test]
fn test_drop_buffered_value_outside_lock() {
    let drops = DropCount::new();
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<DropProbe, Infallible>();
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    sender_holder.replace_value(Some(sender));

    // Closing the pipe drops the buffered values, each of which re-enters the pipe while being
    // dropped. The re-entrant values are dropped as well, because the pipe is closed by then.
    drop(observable);
    assert_eq!(drops.get(), 4);
    assert!(sender_holder.take_value().unwrap().is_disposed());
}

#[test]
fn test_drop_replayed_value_outside_lock() {
    let drops = DropCount::new();
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<DropProbe, Infallible>();
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    sender_holder.replace_value(Some(sender));

    // The observer drops each replayed value, which re-enters the pipe while the replay is still
    // running. The re-entrant values are then replayed and dropped in the same way.
    let _subscription = observable.subscribe_with_callback(drop, |_| {});
    assert_eq!(drops.get(), 4);
}

#[test]
fn test_drop_delivered_value_outside_lock() {
    let drops = DropCount::new();
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<DropProbe, Infallible>();
    let _subscription = observable.subscribe_with_callback(drop, |_| {});

    // The observer drops the value while it is being delivered to it, which re-enters the pipe
    // from inside that delivery. Sending consumes the sender exclusively, so the re-entrant value
    // finds no sender to send with: the pipe cannot be fed from inside a delivery of its own, and
    // the observer is reached once per value.
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    assert_eq!(drops.get(), 1);

    // The observer was parked back, so the pipe keeps working.
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_continue()
    );
    assert_eq!(drops.get(), 2);
    assert!(!sender.is_disposed());
}

#[test]
fn test_drop_delivered_value_after_unsubscribe_outside_lock() {
    let drops = DropCount::new();
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<DropProbe, Infallible>();

    // The value is dropped by the observer, which disposes the subscription while that value is
    // still being delivered: the pipe closes while the observer is out of the state, so the send
    // that is running is what drops the observer, outside the lock.
    let subscription = Shared::new(Mutable::new(None));
    let subscription_cloned = subscription.clone();
    subscription.replace_value(Some(
        observable
            .hook_on_next(move |downstream: &mut _, value| {
                let flow = Observer::on_next(downstream, value);
                if let Some(subscription) = subscription_cloned.take_value() {
                    Disposable::dispose(subscription);
                }
                flow
            })
            .subscribe_with_callback(drop, |_| {}),
    ));

    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_stop()
    );
    assert_eq!(drops.get(), 1);
    assert!(sender.is_disposed());

    // The pipe is closed, so the value is dropped instead of being delivered, still outside the
    // lock.
    assert!(
        sender
            .on_next(reentrant_probe(&sender_holder, &drops))
            .is_stop()
    );
    assert_eq!(drops.get(), 2);
}

#[test]
fn test_non_clone() {
    // Make sure the pipe works when neither the item nor the error is `Clone`.
    let (mut sender, observable) = unicast_subject::<TestStruct, TestStruct>();
    assert!(sender.on_next(TestStruct).is_continue());
    let _subscription = observable.subscribe_with_callback(TestStruct::consume, |_| {});
    sender.on_termination(Termination::Error(TestStruct));
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(&life_marker).is_continue());
        let (_sender, observable) = unicast_subject();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_sender, observable) = unicast_subject::<i32, String>();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_sender, observable) = unicast_subject::<i32, String>();

    observable.filter(|_| true);
}

#[test]
fn test_panicking_replay_closes_the_pipe() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    let (mut sender, observable) = unicast_subject::<Option<PanicOnDrop>, Infallible>();

    expect_panic_on_drop(|value| {
        // Both values are buffered, so subscribing replays them while the pipe still holds the
        // observer on the replay stack. The first one panics when the callback drops it, and the
        // second one carries no payload, so closing the pipe can drop what is left of the queue.
        assert!(sender.on_next(Some(value)).is_continue());
        assert!(sender.on_next(None).is_continue());
        let _subscription = observable.subscribe_with_callback(|_value| {}, |_termination| {});
    });

    // The panic took the observer away with it, so the pipe is over instead of queuing the events
    // that follow for a replay that will never resume.
    assert!(sender.is_disposed());
    assert!(sender.on_next(None).is_stop());
    assert!(sender.is_disposed());
}
