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
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::serialized_multicast::{Admission, MulticastDisposal, SerializedMulticast};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Remembers only the last emission and replays it on completion.
///
/// Unlike [`PublishSubject`](super::publish_subject::PublishSubject), an observer that subscribes after the subject completed still
/// observes that last value, followed by the completion. An error replays nothing.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncSubject<'or, T, E>(SerializedMulticast<'or, T, E, Option<T>>);

impl<T, E> AsyncSubject<'_, T, E> {
    pub fn new() -> Self {
        Self(SerializedMulticast::idle(None))
    }
}

impl<T, E> Default for AsyncSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<MulticastDisposal<'or, T, E, Option<T>>>,
);

impl<'or, T, E> Observable<'or, T, E> for AsyncSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
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

impl<T, E> Observer<T, E> for AsyncSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.update(|last, terminated| {
            if terminated.is_some() {
                // Replacing the value now would change what the later subscribers observe, after
                // the completion already replayed the previous one.
                return UpdateOutcome::empty()
                    .with_drop_outside(Some(value))
                    .without_events();
            }
            // The replaced value is dropped outside the lock: dropping it can run arbitrary code.
            let previous = last.replace(value);
            UpdateOutcome::empty()
                .with_drop_outside(previous)
                .without_events()
        });
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

impl<'or, T, E> Subject<'or, T, E> for AsyncSubject<'or, T, E>
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
