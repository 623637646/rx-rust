//! A subject that remembers only its last value and replays it on completion.
//!
//! The last value lives in the [`SerializedMulticast`]'s resources, so reading it, reading the
//! termination and emitting are one atomic step: the value that is replayed on completion is
//! exactly the one every later subscriber observes, and a value that arrives once the completion
//! was queued is dropped rather than replacing it.

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

/// A subject that emits only its last value, when it completes.
///
/// Values are remembered, not forwarded: on completion the last one is delivered to every
/// observer, including those that subscribe afterwards, followed by the completion. An error
/// delivers no value.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::async_subject::AsyncSubject,
/// };
///
/// let mut seen = Vec::new();
/// let mut sender = AsyncSubject::<i32, std::convert::Infallible, rx_rust::thread_mode::Local>::local();
/// let subscription = sender.clone().subscribe_with_callback(|value| seen.push(value), |_| {});
///
/// let _ = sender.on_next(1);
/// let _ = sender.on_next(2); // Nothing is delivered until the completion.
/// sender.on_termination(Termination::Completed);
/// drop(subscription);
/// assert_eq!(seen, [2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct AsyncSubject<'or, T, E, M: ObserverMode>(
    #[educe(Debug(ignore))] SerializedMulticast<'or, T, E, M, Option<T>>,
);

impl<T, E, M: ObserverMode> AsyncSubject<'_, T, E, M> {
    /// Creates a subject in the mode `M`, for code that is generic over the mode, such as an
    /// operator that creates it in the mode of its source; [`local`](Self::local) and
    /// [`shared`](Self::shared) name the mode instead.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(SerializedMulticast::idle(None))
    }
}

impl<T, E> AsyncSubject<'_, T, E, Local> {
    /// Creates a subject that stays on one thread.
    pub fn local() -> Self {
        Self::new()
    }
}

impl<T, E> AsyncSubject<'_, T, E, Shared> {
    /// Creates a subject that can be fed and subscribed to from any thread.
    pub fn shared() -> Self {
        Self::new()
    }
}

delegate_disposal!(
    Disposal<'or, T, E, M>,
    OptionDisposal<MulticastDisposal<'or, T, E, M, Option<T>>>,
    where M: ObserverMode
);

impl<'or, T, E, M: ObserverMode> ObservableTypes for AsyncSubject<'or, T, E, M>
where
    T: Clone,
    E: Clone,
{
    type Item = T;
    type Error = E;
    type Mode = M;
    type D = Disposal<'or, T, E, M>;
}

impl<'or, T, E, M, OR> Observable<OR> for AsyncSubject<'or, T, E, M>
where
    M: ObserverMode,
    OR: IntoBoxedObserver<'or, T, E, M>,
    T: Clone,
    E: Clone,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        match self
            .0
            .subscribe_with(observer, |last, terminated| match terminated {
                // Reading the last value and the termination is one step, so a subscriber can never
                // observe one without the other.
                None => Admission::Join(Vec::new()),
                Some(Termination::Completed) => Admission::Terminated(
                    last.clone().into_iter().collect(),
                    Termination::Completed,
                ),
                Some(termination) => Admission::Terminated(Vec::new(), termination.clone()),
            }) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E, M: ObserverMode> Observer<T, E> for AsyncSubject<'_, T, E, M>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|last, terminated| {
                if terminated.is_some() {
                    // Replacing the value now would change what the later subscribers observe,
                    // after the completion already replayed the previous one.
                    return UpdateOutcome::new(Flow::Stop)
                        .with_drop_outside(Some(value))
                        .without_events();
                }
                // The replaced value is dropped outside the lock: dropping it can run arbitrary
                // code.
                let previous = last.replace(value);
                UpdateOutcome::new(Flow::Continue)
                    .with_drop_outside(previous)
                    .without_events()
            })
            .unwrap_or(Flow::Stop)
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|last, terminated| {
            if terminated.is_some() {
                return UpdateOutcome::empty()
                    .with_drop_outside(Some(termination))
                    .without_events();
            }
            // The last value and the completion are queued together, and the subject is terminated
            // by that very step: nothing can slip between them.
            let events = if matches!(termination, Termination::Completed) {
                match last.clone() {
                    Some(value) => EventBatch::NextAndTermination(value, termination),
                    None => EventBatch::Termination(termination),
                }
            } else {
                // An error replays nothing, so the last value is not even cloned.
                EventBatch::Termination(termination)
            };
            UpdateOutcome::empty()
                .without_drop_outside()
                .with_events(events)
        });
    }
}

impl<T, E, M: ObserverMode> Subject<T, E> for AsyncSubject<'_, T, E, M>
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
