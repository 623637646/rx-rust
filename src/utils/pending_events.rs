use crate::observer::{Event, Termination};
use educe::Educe;
use std::collections::VecDeque;

/// One atomic batch of events to queue.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum EventBatch<T, E> {
    Next(T),
    Termination(Termination<E>),
    NextAndTermination(T, Termination<E>),
    NextBatch(Vec<T>),
    NextBatchAndTermination(Vec<T>, Termination<E>),
}

impl<T, E> EventBatch<T, E> {
    /// Returns whether the batch carries a termination, after which nothing can be queued.
    pub fn ends_stream(&self) -> bool {
        matches!(
            self,
            Self::Termination(_) | Self::NextAndTermination(..) | Self::NextBatchAndTermination(..)
        )
    }
}

/// The events waiting to be delivered to an observer.
///
/// A termination is the last event of a stream, so it is queued last and nothing is queued after
/// it. Queuing anything after it gives the event back instead of accepting it: the caller drops it
/// outside of the lock that guards this queue, because dropping a value can run arbitrary code
/// that re-enters that lock.
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

    /// Splits `events` into the value to deliver first and the queue holding what follows it.
    ///
    /// The queue is built here, so it is never already terminated and nothing can be given back.
    /// The first value is handed to the caller instead of being queued, so a batch carrying a
    /// single value leaves the queue empty and never allocates it.
    pub fn from_batch(events: EventBatch<T, E>) -> (Option<T>, Self) {
        let empty_queue = |termination| Self {
            values: VecDeque::new(),
            termination,
        };
        // A batch of many values is turned into the queue directly, which reuses its allocation,
        // and the first value is then taken off the front.
        let queue_batch = |values: Vec<T>, termination| {
            let mut values = VecDeque::from(values);
            let first_next = values.pop_front();
            (
                first_next,
                Self {
                    values,
                    termination,
                },
            )
        };
        match events {
            EventBatch::Next(value) => (Some(value), empty_queue(None)),
            EventBatch::Termination(termination) => (None, empty_queue(Some(termination))),
            EventBatch::NextAndTermination(value, termination) => {
                (Some(value), empty_queue(Some(termination)))
            }
            EventBatch::NextBatch(values) => queue_batch(values, None),
            EventBatch::NextBatchAndTermination(values, termination) => {
                queue_batch(values, Some(termination))
            }
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

    /// Queues `event`, or gives it back when the last event has already been queued.
    #[must_use = "a rejected event must be dropped outside the lock that guards these events"]
    pub fn push(&mut self, event: Event<T, E>) -> Option<Event<T, E>> {
        if self.is_terminated() {
            return Some(event);
        }
        match event {
            Event::Next(value) => self.values.push_back(value),
            Event::Termination(termination) => self.termination = Some(termination),
        }
        None
    }

    /// Queues `events`, or gives them back when the last event has already been queued.
    ///
    /// A batch is queued as a whole: it holds at most one termination and queues it last, so no
    /// event of a batch can be rejected on its own.
    #[must_use = "rejected events must be dropped outside the lock that guards these events"]
    pub fn push_batch(&mut self, events: EventBatch<T, E>) -> Option<EventBatch<T, E>> {
        if self.is_terminated() {
            return Some(events);
        }
        match events {
            EventBatch::Next(value) => self.values.push_back(value),
            EventBatch::Termination(termination) => self.termination = Some(termination),
            EventBatch::NextAndTermination(value, termination) => {
                self.values.push_back(value);
                self.termination = Some(termination);
            }
            EventBatch::NextBatch(values) => self.values.extend(values),
            EventBatch::NextBatchAndTermination(values, termination) => {
                self.values.extend(values);
                self.termination = Some(termination);
            }
        }
        None
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
