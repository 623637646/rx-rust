//! A subject that buffers what it emits and replays that buffer to every new subscriber.
//!
//! The buffer lives in the [`SerializedMulticast`]'s resources, so buffering a value and
//! forwarding it are one atomic step, and so are snapshotting the buffer and joining the
//! multicast: a subscriber can neither miss a value emitted while it was subscribing nor observe
//! one twice.

use super::Subject;
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::serialized_multicast::{Admission, MulticastDisposal, SerializedMulticast};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Buffers emissions and replays them to late subscribers.
///
/// A subscriber that arrives after the subject terminated observes the buffered values followed by
/// the termination, whether the subject completed or errored.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ReplaySubject<'or, T, E>(SerializedMulticast<'or, T, E, Buffer<T>>);

/// The replayed values, and how many of them are kept.
#[derive(Educe)]
#[educe(Debug)]
struct Buffer<T> {
    values: VecDeque<T>,
    /// The number of values kept, or [`None`] to keep every one of them.
    size: Option<usize>,
}

impl<T, E> ReplaySubject<'_, T, E> {
    pub fn new(buffer_size: Option<usize>) -> Self {
        let values = match buffer_size {
            Some(size) => VecDeque::with_capacity(size),
            None => VecDeque::new(),
        };
        Self(SerializedMulticast::idle(Buffer {
            values,
            size: buffer_size,
        }))
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<MulticastDisposal<'or, T, E, Buffer<T>>>,
);

impl<'or, T, E> Observable<'or, T, E> for ReplaySubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match self.0.subscribe_with(observer, |buffer, terminated| {
            // The buffer is the history of the subject, so it is replayed whichever way the
            // subject terminated: an error does not erase what was emitted before it.
            let values = buffer.values.iter().cloned().collect();
            match terminated {
                None => Admission::Join(values),
                Some(termination) => Admission::Terminated(values, termination.clone()),
            }
        }) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E> Observer<T, E> for ReplaySubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.update(|buffer, terminated| {
            if terminated.is_some() {
                return UpdateOutcome::empty()
                    .with_drop_outside(Some(value))
                    .without_events();
            }
            // Buffering the value and forwarding it are one step, so the buffer a subscriber is
            // replayed always matches the values it then receives. The evicted value is dropped
            // outside the lock: dropping it can run arbitrary code.
            let evicted = buffer.push(value.clone());
            UpdateOutcome::empty()
                .with_drop_outside(evicted)
                .with_next_event(value)
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send(EventBatch::Termination(termination));
    }
}

impl<T> Buffer<T> {
    /// Buffers `value`, returning the value it evicted, if any.
    ///
    /// A buffer of size zero keeps nothing: the value is only forwarded.
    fn push(&mut self, value: T) -> Option<T> {
        let Some(size) = self.size else {
            self.values.push_back(value);
            return None;
        };
        if self.values.len() < size {
            self.values.push_back(value);
            return None;
        }
        let evicted = self.values.pop_front();
        if evicted.is_some() {
            self.values.push_back(value);
        }
        evicted
    }
}

impl<'or, T, E> Subject<'or, T, E> for ReplaySubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.0.terminated()
    }
}
