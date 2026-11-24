use super::SerializedMulticast;
use crate::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observer::{EventBatch, Flow, Termination, callback_observer::CallbackObserver},
    thread_mode::{
        Local,
        mutable::{MutableExt, MutableHelper},
    },
    utils::serialized_delivery::UpdateOutcome,
};
use std::sync::{Arc, Mutex};

/// Two observers share one delivery in subscription order, a disposed one stops receiving, and
/// a termination stops the multicast.
#[test]
fn test_two_observers_and_a_disposal() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let multicast = SerializedMulticast::<_, (), Local, _>::idle(());

    let seen_by_first = Arc::clone(&seen);
    let first = multicast.clone().subscribe(CallbackObserver::new(
        move |value| seen_by_first.with_mut(|seen| seen.push(("first", value))),
        |_| {},
    ));
    let seen_by_second = Arc::clone(&seen);
    let _second = multicast.clone().subscribe(CallbackObserver::new(
        move |value| seen_by_second.with_mut(|seen| seen.push(("second", value))),
        |_| {},
    ));

    assert_eq!(multicast.send(EventBatch::Next(1)), Flow::Continue(()));
    first.dispose();
    assert_eq!(multicast.send(EventBatch::Next(2)), Flow::Continue(()));
    assert_eq!(
        multicast.send(EventBatch::Termination(Termination::Completed)),
        Flow::Break(())
    );
    assert_eq!(
        seen.clone_value(),
        [("first", 1), ("second", 1), ("second", 2)]
    );
}

/// An update that arrives after the termination does not run, and is dropped outside the lock.
#[test]
fn test_update_after_termination() {
    let multicast = SerializedMulticast::<i32, (), Local, _>::idle(());
    let _ = multicast.send(EventBatch::Termination(Termination::Completed));

    // Takes the lock as it is dropped.
    let seen = Arc::new(Mutex::new(None));
    let (multicast_of_drop, seen_by_drop) = (multicast.clone(), seen.clone());
    let captured = DisposeOnDrop::new(CallbackDisposal::new(move || {
        seen_by_drop.replace_value(multicast_of_drop.termination());
    }));

    let flow = multicast.update(move |()| {
        drop(captured); // Never runs: it is what makes the update own the value.
        UpdateOutcome::empty().with_next_event(1)
    });
    assert_eq!(flow, Flow::Break(()));
    assert_eq!(seen.clone_value(), Some(Termination::Completed));
}

/// An observer that subscribes after the termination is terminated at once, and its disposal
/// holds nothing of the multicast.
#[test]
fn test_subscribe_after_termination() {
    let terminations = Arc::new(Mutex::new(Vec::new()));
    let multicast = SerializedMulticast::<i32, (), Local, _>::idle(());
    assert_eq!(
        multicast.send(EventBatch::Termination(Termination::Completed)),
        Flow::Break(())
    );

    let terminations_seen = Arc::clone(&terminations);
    let disposal = multicast.subscribe(CallbackObserver::new(
        |_| -> () { unreachable!() },
        move |termination| terminations_seen.with_mut(|seen| seen.push(termination)),
    ));

    assert_eq!(terminations.clone_value(), [Termination::Completed]);
    assert!(disposal.0.is_none());
    disposal.dispose();
}
