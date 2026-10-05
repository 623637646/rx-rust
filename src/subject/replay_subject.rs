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
use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::thread_mode::{Local, Shared};
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::serialized_multicast::{Admission, MulticastDisposal, SerializedMulticast};
use crate::{
    observable::{Observable, ObservableTypes},
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
/// let subject = ReplaySubject::<i32, std::convert::Infallible, rx_rust::thread_mode::Local>::local(Some(2));
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
#[educe(Debug, Clone(bound()))]
pub struct ReplaySubject<'or, T, E, M: ObserverMode>(
    #[educe(Debug(ignore))] SerializedMulticast<'or, T, E, M, Buffer<T>>,
);

/// The replayed values, and how many of them are kept.
#[derive(Educe)]
#[educe(Debug)]
struct Buffer<T> {
    values: VecDeque<T>,
    /// The number of values kept, or [`None`] to keep every one of them.
    size: Option<usize>,
}

impl<T, E, M: ObserverMode> ReplaySubject<'_, T, E, M> {
    /// Creates a subject in the mode `M`, for code that is generic over the mode, such as an operator
    /// that creates it in the mode of its source; [`local`](Self::local) and [`shared`](Self::shared)
    /// name the mode instead.
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

impl<T, E> ReplaySubject<'_, T, E, Local> {
    /// Creates a subject that stays on one thread.
    pub fn local(buffer_size: Option<usize>) -> Self {
        Self::new(buffer_size)
    }
}

impl<T, E> ReplaySubject<'_, T, E, Shared> {
    /// Creates a subject that can be fed and subscribed to from any thread.
    pub fn shared(buffer_size: Option<usize>) -> Self {
        Self::new(buffer_size)
    }
}

delegate_disposal!(
    Disposal<'or, T, E, M>,
    OptionDisposal<MulticastDisposal<'or, T, E, M, Buffer<T>>>,
    where M: ObserverMode
);

impl<'or, T, E, M: ObserverMode> ObservableTypes for ReplaySubject<'or, T, E, M>
where
    T: Clone,
    E: Clone,
{
    type Item = T;
    type Error = E;
    type Mode = M;
    type D = Disposal<'or, T, E, M>;
}

impl<'or, T, E, M, OR> Observable<OR> for ReplaySubject<'or, T, E, M>
where
    M: ObserverMode,
    OR: IntoBoxedObserver<'or, T, E, M>,
    T: Clone,
    E: Clone,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
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

impl<T, E, M: ObserverMode> Observer<T, E> for ReplaySubject<'_, T, E, M>
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

impl<T, E, M: ObserverMode> Subject<T, E> for ReplaySubject<'_, T, E, M>
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
