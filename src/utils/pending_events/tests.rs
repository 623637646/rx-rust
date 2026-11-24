use super::PendingEvents;
use crate::observer::{Event, Termination};
use std::collections::VecDeque;

#[test]
fn test_take_values_into_leaves_the_termination() {
    let mut pending = PendingEvents::<_, ()>::new();
    let mut batch = VecDeque::new();
    assert!(!pending.take_values_into(&mut batch));

    for event in [
        Event::Next(1),
        Event::Next(2),
        Event::Termination(Termination::Completed),
    ] {
        assert!(pending.push(event).is_none());
    }
    assert!(pending.take_values_into(&mut batch));
    assert_eq!(batch, [1, 2]);

    // The termination stays queued, so nothing can be queued after it still.
    assert!(pending.is_terminated());
    assert!(pending.push(Event::Next(3)).is_some());
    batch.clear();
    assert!(!pending.take_values_into(&mut batch));
    assert_eq!(pending.take_termination(), Some(Termination::Completed));
    assert!(!pending.is_terminated());
}

#[test]
fn test_nothing_after_termination() {
    let mut pending = PendingEvents::<_, ()>::new();
    assert!(pending.push(Event::Next(1)).is_none());
    assert!(
        pending
            .push(Event::Termination(Termination::Completed))
            .is_none()
    );
    // Nothing can follow a termination: the event is handed back to be dropped elsewhere.
    assert!(pending.push(Event::Next(2)).is_some());

    let mut batch = VecDeque::new();
    assert!(pending.take_values_into(&mut batch));
    assert_eq!(batch, [1]);
    assert_eq!(pending.take_termination(), Some(Termination::Completed));
    assert_eq!(pending.take_termination(), None);
}
