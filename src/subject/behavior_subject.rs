//! A subject that keeps a current value and hands it to every new subscriber.
//!
//! The current value lives in the [`SerializedMulticast`]'s resources, so replacing it and
//! forwarding it are one atomic step, and so are reading it and joining the multicast: a
//! subscriber can neither miss a value emitted while it was subscribing nor observe one twice.

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

/// Keeps the latest value and emits it immediately to new subscribers.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorSubject<'or, T, E>(SerializedMulticast<'or, T, E, T>);

impl<T, E> BehaviorSubject<'_, T, E> {
    pub fn new(value: T) -> Self {
        Self(SerializedMulticast::idle(value))
    }
}

impl<T, E> BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    /// The current value.
    ///
    /// # Panics
    ///
    /// Panics once an observer's callback has panicked, which takes the subject's whole state with
    /// it and leaves nothing to return.
    pub fn value(&self) -> T {
        self.0
            .read(|value, _| value.clone())
            .expect("the subject is dead because an observer panicked")
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<MulticastDisposal<'or, T, E, T>>,
);

impl<'or, T, E> Observable<'or, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match self
            .0
            .subscribe_with(observer, |value, terminated| match terminated {
                // The current value is snapshotted under the very lock that queues the subscription,
                // so it is followed by exactly the values emitted after it.
                None => Admission::Join(vec![value.clone()]),
                Some(termination) => Admission::Terminated(Vec::new(), termination.clone()),
            }) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|current, terminated| {
                if terminated.is_some() {
                    return UpdateOutcome::new(Flow::Stop)
                        .with_drop_outside(Some(value))
                        .without_events();
                }
                // Replacing the current value and forwarding it are one step, so the value a
                // subscriber is given always matches the values it then receives. The replaced
                // value is dropped outside the lock: dropping it can run arbitrary code.
                let previous = std::mem::replace(current, value.clone());
                UpdateOutcome::new(Flow::Continue)
                    .with_drop_outside(Some(previous))
                    .with_next_event(value)
            })
            .unwrap_or(Flow::Stop)
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.send(EventBatch::Termination(termination));
    }
}

impl<'or, T, E> Subject<'or, T, E> for BehaviorSubject<'or, T, E>
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
