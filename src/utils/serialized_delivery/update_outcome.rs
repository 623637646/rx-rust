//! What a host and a [`SerializedDelivery`](super::SerializedDelivery) hand each other across an
//! update: the [`UpdateOutcome`] its callback gives back under the lock, and the [`Updated`] the
//! update answers.
//!
//! Its own module keeps the fields of an outcome out of reach of the delivery, so that the
//! type-state methods are the only way to build one: each effect is set at most once, and an
//! outcome whose events are still undecided has none. The delivery takes an outcome apart with
//! `into_parts` only.

use crate::observer::{EventBatch, Flow, Termination};
use educe::Educe;
use std::convert::identity;

/// What an update answers: the result its callback gave back, and whether the observer still
/// accepts events — or that the callback did not run at all, because the delivery had stopped.
///
/// An `on_next` built on updates answers the [`flow`](Self::flow) of its last one; a step that only
/// needs the result, a reservation say, takes it with [`result`](Self::result) and leaves the flow
/// to the update that ends the work. A step that needs both matches on the variants.
#[derive(Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "an `on_next` answers the flow of its last update"]
pub enum Updated<R> {
    /// The callback ran and gave back `result`.
    ///
    /// `flow` is what delivering the events it queued answered or, when it queued none, whether
    /// the delivery still accepts events: [`Flow::Stop`] once a termination is queued behind a
    /// delivery running elsewhere. An outcome that [answers a stop](UpdateOutcome::answering_stop)
    /// makes it [`Flow::Stop`] whatever the delivery answered.
    Ran {
        /// What the callback gave back with [`UpdateOutcome::new`].
        result: R,
        /// Whether the caller may go on sending.
        flow: Flow,
    },
    /// The delivery had stopped, so the callback did not run: it was dropped, with everything it
    /// captured, outside the lock.
    ///
    /// Stopping is terminal: once a delivery has stopped, every later update answers this.
    Stopped,
}

impl<R> Updated<R> {
    /// Whether the caller may go on sending: [`Flow::Stop`] for a delivery that had stopped.
    pub fn flow(&self) -> Flow {
        match self {
            Self::Ran { flow, .. } => *flow,
            Self::Stopped => Flow::Stop,
        }
    }

    /// The result of the callback, whatever the flow, or [`None`] when it did not run.
    pub fn result(self) -> Option<R> {
        match self {
            Self::Ran { result, .. } => Some(result),
            Self::Stopped => None,
        }
    }
}

/// The marker of an [`UpdateOutcome`] that has not decided what to drop outside the lock yet.
#[derive(Debug)]
pub struct DropUndecided;

/// The marker of an [`UpdateOutcome`] that has decided, carrying the value to drop, if any.
#[derive(Educe)]
#[educe(Debug)]
pub struct DropDecided<T>(Option<T>);

/// What an update performed under the lock produced: the events to queue, a value to drop outside
/// the lock, the result to give back to the caller, and whether the caller must stop sending.
///
/// The two type-state parameters make each effect settable at most once, and let the arms of a
/// branching update share one type: an arm that sets no events next to one that does still has to
/// say so, with [`Self::without_events`].
#[derive(Educe)]
#[educe(Debug)]
pub struct UpdateOutcome<T, E, R = (), DO = DropUndecided, const EVENTS_DECIDED: bool = false> {
    events: Option<EventBatch<T, E>>,
    drop_outside: DO,
    /// [`Flow::Stop`] once [`Self::answering_stop`] was called.
    flow: Flow,
    result: R,
}

impl<T, E, R> UpdateOutcome<T, E, R> {
    /// An outcome that gives `result` back to the caller of the update, with the events and the
    /// value to drop still to be decided.
    pub fn new(result: R) -> Self {
        Self {
            events: None,
            drop_outside: DropUndecided,
            flow: Flow::Continue,
            result,
        }
    }
}

impl<T, E> UpdateOutcome<T, E> {
    /// [`Self::new`] with no result.
    pub fn empty() -> Self {
        Self::new(())
    }
}

impl<T, E, R, DO, const EVENTS_DECIDED: bool> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED> {
    /// Rebuilds the outcome from its two effects, mapped, keeping its flow and result: what moves
    /// it from one type state to another.
    fn map_effects<T2, DO2, const EVENTS_DECIDED2: bool>(
        self,
        events: impl FnOnce(Option<EventBatch<T, E>>) -> Option<EventBatch<T2, E>>,
        drop_outside: impl FnOnce(DO) -> DO2,
    ) -> UpdateOutcome<T2, E, R, DO2, EVENTS_DECIDED2> {
        UpdateOutcome {
            events: events(self.events),
            drop_outside: drop_outside(self.drop_outside),
            flow: self.flow,
            result: self.result,
        }
    }

    /// Translates the events, keeping every other effect and the type state as they are.
    ///
    /// This is how [`SerializedMulticast`](crate::utils::serialized_multicast::SerializedMulticast)
    /// turns the outcome of its own host into one over the events of the delivery underneath it,
    /// under the very lock the host's outcome was produced under.
    pub(crate) fn map_events<T2>(
        self,
        map: impl FnOnce(EventBatch<T, E>) -> EventBatch<T2, E>,
    ) -> UpdateOutcome<T2, E, R, DO, EVENTS_DECIDED> {
        self.map_effects(|events| events.map(map), identity)
    }

    /// Takes the outcome apart for the delivery that ran the update: the events to queue, the
    /// value to drop outside the lock, the flow the host answers, and the result, in that order.
    pub(super) fn into_parts(self) -> (Option<EventBatch<T, E>>, DO, Flow, R) {
        (self.events, self.drop_outside, self.flow, self.result)
    }

    /// Makes the update answer [`Flow::Stop`], whatever delivering its events answers.
    ///
    /// This is the host deciding that the caller must send nothing more, while the delivery goes
    /// on for the others: an inner source that has been superseded, say. It can only tighten the
    /// flow: there is no way to answer [`Flow::Continue`] for a delivery that stopped.
    pub fn answering_stop(self) -> Self {
        Self {
            flow: Flow::Stop,
            ..self
        }
    }
}

impl<T, E, R, const EVENTS_DECIDED: bool> UpdateOutcome<T, E, R, DropUndecided, EVENTS_DECIDED> {
    /// Drops `drop_outside` after releasing the lock and sending or queueing the events.
    /// Events queued behind a running delivery may be delivered after this value is dropped.
    /// This is where a value the update displaced, or rejected, goes.
    pub fn with_drop_outside<DO>(
        self,
        drop_outside: DO,
    ) -> UpdateOutcome<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        self.map_effects(identity, |DropUndecided| DropDecided(Some(drop_outside)))
    }

    /// States that nothing needs to be dropped outside the lock.
    pub fn without_drop_outside<DO>(
        self,
    ) -> UpdateOutcome<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        self.map_effects(identity, |DropUndecided| DropDecided(None))
    }
}

impl<T, E, R, DO> UpdateOutcome<T, E, R, DO, false> {
    /// Queues one value.
    pub fn with_next_event(self, next: T) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::Next(next))
    }

    /// Queues the termination, after which nothing more can be queued.
    pub fn with_termination_event(
        self,
        termination: Termination<E>,
    ) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::Termination(termination))
    }

    /// Queues a last value and then the termination.
    pub fn with_next_and_termination_events(
        self,
        next: T,
        termination: Termination<E>,
    ) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::NextAndTermination(next, termination))
    }

    /// Queues `events` as one unit.
    pub fn with_events(self, events: EventBatch<T, E>) -> UpdateOutcome<T, E, R, DO, true> {
        // Events can be set only while undecided, so the ones replaced here are always `None`.
        self.map_effects(|_| Some(events), identity)
    }

    /// States that the update queues nothing.
    pub fn without_events(self) -> UpdateOutcome<T, E, R, DO, true> {
        self.map_effects(|_| None, identity)
    }
}
