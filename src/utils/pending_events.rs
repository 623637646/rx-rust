//! The queue of events waiting behind a running delivery, fed one [`Event`] at a time.

use crate::observer::{Event, Termination};
use educe::Educe;
use std::collections::VecDeque;

/// The events waiting to be delivered to an observer.
///
/// A termination is the last event of a stream, so it is queued last and nothing is queued after
/// it. Queuing anything after it gives the event back instead of accepting it: the caller drops it
/// outside of the lock that guards this queue, because dropping a value can run arbitrary code
/// that re-enters that lock.
#[derive(Educe)]
#[educe(Debug, Default)]
pub(crate) struct PendingEvents<T, E> {
    values: VecDeque<T>,
    termination: Option<Termination<E>>,
}

impl<T, E> PendingEvents<T, E> {
    /// An empty queue.
    pub(crate) fn new() -> Self {
        Self {
            values: VecDeque::new(),
            termination: None,
        }
    }

    /// Returns whether the last event has been queued, after which nothing can be queued anymore.
    pub(crate) fn is_terminated(&self) -> bool {
        self.termination.is_some()
    }

    /// Queues `event`, or gives it back when the last event has already been queued.
    #[must_use = "a rejected event must be dropped outside the lock that guards these events"]
    pub(crate) fn push(&mut self, event: Event<T, E>) -> Option<Event<T, E>> {
        if self.is_terminated() {
            return Some(event);
        }
        match event {
            Event::Next(value) => self.values.push_back(value),
            Event::Termination(termination) => self.termination = Some(termination),
        }
        None
    }

    /// Moves every queued value into `batch`, which must be empty, leaving the termination in
    /// place, and returns whether there was any. The queue keeps the allocation of `batch` in
    /// exchange, so a loop that keeps taking batches goes back and forth between two buffers
    /// instead of allocating one per batch.
    pub(crate) fn take_values_into(&mut self, batch: &mut VecDeque<T>) -> bool {
        debug_assert!(
            batch.is_empty(),
            "a batch is taken once the last one is done"
        );
        if self.values.is_empty() {
            return false;
        }
        std::mem::swap(&mut self.values, batch);
        true
    }

    /// Takes the last event, whether or not values are still queued before it.
    pub(crate) fn take_termination(&mut self) -> Option<Termination<E>> {
        self.termination.take()
    }
}

#[cfg(test)]
mod tests {
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
}
