use crate::observer::{Event, Termination};
use educe::Educe;
use std::collections::VecDeque;

/// The events waiting to be delivered to an observer.
///
/// A termination is the last event of a stream, so it is queued last and nothing is queued after
/// it. Callers must not queue an event once [`PendingEvents::is_terminated`] holds; they must drop
/// it instead, outside of the lock that guards these events, because dropping a value can run
/// arbitrary code that re-enters that lock.
#[derive(Educe)]
#[educe(Debug)]
pub struct PendingEvents<T, E> {
    values: VecDeque<T>,
    termination: Option<Termination<E>>,
}

impl<T, E> PendingEvents<T, E> {
    pub fn new() -> Self {
        Self {
            values: VecDeque::new(),
            termination: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(capacity),
            termination: None,
        }
    }

    /// Returns whether the last event has been queued, after which nothing can be queued anymore.
    pub fn is_terminated(&self) -> bool {
        self.termination.is_some()
    }

    /// Returns whether there is nothing left to deliver.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.termination.is_none()
    }

    /// Queues a value. Must not be called once [`PendingEvents::is_terminated`] holds.
    pub fn push_next(&mut self, value: T) {
        debug_assert!(!self.is_terminated());
        self.values.push_back(value);
    }

    /// Queues several values. Must not be called once [`PendingEvents::is_terminated`] holds.
    pub fn extend_next(&mut self, values: impl IntoIterator<Item = T>) {
        debug_assert!(!self.is_terminated());
        self.values.extend(values);
    }

    /// Queues the last event. Must not be called once [`PendingEvents::is_terminated`] holds.
    pub fn set_termination(&mut self, termination: Termination<E>) {
        debug_assert!(!self.is_terminated());
        self.termination = Some(termination);
    }

    /// Takes the next event to deliver, which is the termination once no value is left.
    pub fn pop(&mut self) -> Option<Event<T, E>> {
        match self.pop_next() {
            Some(value) => Some(Event::Next(value)),
            None => self.take_termination().map(Event::Termination),
        }
    }

    /// Takes the next value to deliver, leaving the termination in place.
    pub fn pop_next(&mut self) -> Option<T> {
        self.values.pop_front()
    }

    /// Takes the last event, whether or not values are still queued before it.
    pub fn take_termination(&mut self) -> Option<Termination<E>> {
        self.termination.take()
    }
}

impl<T, E> Default for PendingEvents<T, E> {
    fn default() -> Self {
        Self::new()
    }
}
