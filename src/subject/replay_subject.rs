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
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// A subject that buffers what it emits and replays the buffer to every new subscriber.
///
/// The buffer keeps the last `buffer_size` values, or every value when the size is `None`. A
/// subscriber that arrives after the subject terminated receives the buffered values and then the
/// termination, whether the subject completed or errored.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::replay_subject::ReplaySubject,
/// };
///
/// let mut seen = Vec::new();
/// let subject = ReplaySubject::<i32, std::convert::Infallible>::new(Some(2));
/// let mut sender = subject.clone();
/// let _ = sender.on_next(1);
/// let _ = sender.on_next(2);
/// let _ = sender.on_next(3);
/// sender.on_termination(Termination::Completed);
///
/// let subscription = subject.subscribe_with_callback(
///     |value| seen.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
/// drop(subscription);
/// assert_eq!(seen, [2, 3]); // The last two, then the completion.
/// ```
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
    /// Creates a subject that keeps the last `buffer_size` values, or all of them for `None`.
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
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|buffer, terminated| {
                if terminated.is_some() {
                    return UpdateOutcome::new(Flow::Stop)
                        .with_drop_outside(Some(value))
                        .without_events();
                }
                // Buffering the value and forwarding it are one step, so the buffer a subscriber
                // is replayed always matches the values it then receives. The evicted value is
                // dropped outside the lock: dropping it can run arbitrary code.
                let evicted = buffer.push(value.clone());
                UpdateOutcome::new(Flow::Continue)
                    .with_drop_outside(evicted)
                    .with_next_event(value)
            })
            .unwrap_or(Flow::Stop)
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.send(EventBatch::Termination(termination));
    }
}

impl<T> Buffer<T> {
    /// Buffers `value`, returning the value it evicted, if any, for the caller to drop outside
    /// the lock.
    ///
    /// A buffer of size zero keeps nothing: the value is only forwarded, and it is the value
    /// itself that is handed back.
    fn push(&mut self, value: T) -> Option<T> {
        match self.size {
            Some(0) => Some(value),
            Some(size) if self.values.len() >= size => {
                let evicted = self.values.pop_front();
                self.values.push_back(value);
                evicted
            }
            Some(_) | None => {
                self.values.push_back(value);
                None
            }
        }
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
