//! A multicast subject that forwards what it receives to every observer.
//!
//! It is [`SerializedMulticast`] with nothing of its own to guard: every method here is one call
//! into it. The subject is terminated as soon as [`Observer::on_termination`] is *called*, not when
//! the termination reaches the observers — see the multicast's module documentation for why, and
//! for everything else that governs the ordering here.

use super::Subject;
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_multicast::{MulticastDisposal, SerializedMulticast};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Basic multicast subject that forwards events to all observers.
///
/// Observers are notified in subscription order, and an observer that unsubscribes does not
/// disturb the order of the others.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E>(SerializedMulticast<'or, T, E>);

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self(SerializedMulticast::idle(()))
    }
}

impl<T, E> Default for PublishSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<MulticastDisposal<'or, T, E, ()>>,
);

impl<'or, T, E> Observable<'or, T, E> for PublishSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match self.0.subscribe(observer) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        // A value that arrives after the termination is dropped outside the lock.
        self.0.send(EventBatch::Next(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        // Only the first termination is ever queued, and nothing joins the subject after it.
        let _ = self.0.send(EventBatch::Termination(termination));
    }
}

impl<'or, T, E> Subject<'or, T, E> for PublishSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn terminated(&self) -> Option<Termination<E>> {
        self.0.terminated()
    }
}
