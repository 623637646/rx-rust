//! The state machine that serializes the events delivered to one observer, together with the lock
//! that guards it and the only operations allowed to drive it.
//!
//! [`SerializedDelivery`] is a shared handle. Its lock never leaves this module, so a host cannot
//! hold it across a notification, a drop, or a second lock: every transition, the delivery loop,
//! and the rule that nothing is dropped or notified under the lock live here.
//!
//! A host that must change its own data, and emit the resulting events atomically, does so through
//! [`SerializedDelivery::update`], which runs its callback under the same lock that then queues
//! the events. The callback describes its outcome with an [`UpdateOutcome`], the one way to hand
//! this module events to queue and a value to drop once they were delivered.

use crate::{
    observer::{Observer, Termination},
    utils::{
        mutable::{Mutable, MutableExt, MutableHelper},
        on_panic::on_panic,
        pending_events::{EventBatch, PendingEvents},
        types::{Shared, WeakShared},
    },
};
use educe::Educe;

/// A shared, serialized delivery of events to one observer.
///
/// `R` is whatever the host owns alongside the observer. It is dropped, outside the lock, once the
/// delivery stops — after the terminal notification when the delivery stops by terminating.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SerializedDelivery<T, E, OR, R>(Shared<Mutable<State<T, E, OR, R>>>);

/// A non-owning reference to a [`SerializedDelivery`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakSerializedDelivery<T, E, OR, R>(WeakShared<Mutable<State<T, E, OR, R>>>);

#[derive(Educe)]
#[educe(Debug)]
enum State<T, E, OR, R> {
    /// The observer is parked in the state while no delivery is running.
    Idle { observer: OR, resources: R },
    /// The observer is held by the delivery loop, while re-entrant events wait here.
    Delivering {
        pending: PendingEvents<T, E>,
        resources: R,
    },
    /// The observer and all resources are gone. Every later event is rejected.
    Stopped,
}

/// The action to perform after releasing the lock used to call `enqueue_batch`.
enum EnqueueAction<T, E, OR> {
    /// Start a delivery loop with the observer removed from the locked state.
    Start { observer: OR, first_next: Option<T> },
    /// The batch was queued for a running delivery, or was empty and needed no work.
    Accepted,
    /// The delivery was already stopped or a termination was already queued.
    Rejected(EventBatch<T, E>),
}

/// One transition of the delivery loop, computed while the state is locked and acted on outside
/// it. The observer is threaded through, so it is never used or dropped under the lock.
enum Step<T, E, OR, R> {
    /// Deliver one value, then ask for the next step.
    Next(OR, T),
    /// Deliver the termination, then drop `resources`. The state is already `Stopped`.
    Terminate {
        observer: OR,
        termination: Termination<E>,
        resources: R,
    },
    /// Nothing is queued; the observer was parked back into the state.
    Parked,
    /// The delivery stopped; drop the observer outside the lock.
    Stopped(OR),
}

/// Returned when an update did not run because the delivery has stopped.
///
/// Stopping is terminal: once a delivery has stopped, every later update returns this instead of
/// running, and the update and everything it captured are dropped outside the lock.
#[derive(Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryStopped;

/// The marker of an [`UpdateOutcome`] that has not decided what to drop outside the lock yet.
pub struct DropUndecided;

/// The marker of an [`UpdateOutcome`] that has decided, carrying the value to drop, if any.
pub struct DropDecided<T>(Option<T>);

/// What an update performed under the lock produced: the events to queue, a value to drop once
/// they were delivered, and the result to give back to the caller.
///
/// The two type-state parameters make each effect settable at most once, and let the arms of a
/// branching update share one type: an arm that sets no events next to one that does still has to
/// say so, with [`Self::without_events`].
#[derive(Educe)]
#[educe(Debug)]
pub struct UpdateOutcome<T, E, R = (), DO = DropUndecided, const EVENTS_DECIDED: bool = false> {
    events: Option<EventBatch<T, E>>,
    drop_outside: DO,
    result: R,
}

impl<T, E, R> UpdateOutcome<T, E, R> {
    pub fn new(result: R) -> Self {
        Self {
            events: None,
            drop_outside: DropUndecided,
            result,
        }
    }
}

impl<T, E> UpdateOutcome<T, E> {
    pub fn empty() -> Self {
        Self::new(())
    }
}

impl<T, E, R, DO, const EVENTS_DECIDED: bool> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED> {
    /// Takes the outcome apart, for a host that queues the events somewhere else.
    ///
    /// This is how [`SerializedMulticast`](crate::utils::serialized_multicast::SerializedMulticast)
    /// translates the outcome of its own host into the events of the delivery underneath it. The
    /// events must still be queued, and the value still be dropped, under and outside the very
    /// lock this outcome was produced under.
    pub(crate) fn into_parts(self) -> (Option<EventBatch<T, E>>, DO, R) {
        (self.events, self.drop_outside, self.result)
    }
}

impl<T, E, R, const EVENTS_DECIDED: bool> UpdateOutcome<T, E, R, DropUndecided, EVENTS_DECIDED> {
    pub fn with_drop_outside<DO>(
        self,
        drop_outside: DO,
    ) -> UpdateOutcome<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        UpdateOutcome {
            events: self.events,
            drop_outside: DropDecided(Some(drop_outside)),
            result: self.result,
        }
    }

    pub fn without_drop_outside<DO>(
        self,
    ) -> UpdateOutcome<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        UpdateOutcome {
            events: self.events,
            drop_outside: DropDecided(None),
            result: self.result,
        }
    }
}

impl<T, E, R, DO> UpdateOutcome<T, E, R, DO, false> {
    pub fn with_next_event(self, next: T) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::Next(next))
    }

    pub fn with_termination_event(
        self,
        termination: Termination<E>,
    ) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::Termination(termination))
    }

    pub fn with_next_and_termination_events(
        self,
        next: T,
        termination: Termination<E>,
    ) -> UpdateOutcome<T, E, R, DO, true> {
        self.with_events(EventBatch::NextAndTermination(next, termination))
    }

    pub fn with_events(self, events: EventBatch<T, E>) -> UpdateOutcome<T, E, R, DO, true> {
        UpdateOutcome {
            events: Some(events),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    pub fn without_events(self) -> UpdateOutcome<T, E, R, DO, true> {
        UpdateOutcome {
            events: None,
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }
}

impl<T, E, OR, R> SerializedDelivery<T, E, OR, R> {
    /// Starts with the observer attached and parked, waiting for the first event.
    pub fn idle(observer: OR, resources: R) -> Self {
        Self(Shared::new(Mutable::new(State::Idle {
            observer,
            resources,
        })))
    }

    /// Stops the delivery, dropping the observer without notifying it. Stopping again is a no-op.
    pub fn stop(&self) {
        // `Stopped` is the only variant that owns nothing, so replacing the state with it takes
        // the observer, the queued events and the resources out. Binding them here drops all of
        // them outside the lock, avoiding a potential deadlock.
        let _deferred_drop = self.0.replace_value(State::Stopped);
    }

    pub fn downgrade(&self) -> WeakSerializedDelivery<T, E, OR, R> {
        WeakSerializedDelivery(Shared::downgrade(&self.0))
    }
}

impl<T, E, OR, R> SerializedDelivery<T, E, OR, R>
where
    OR: Observer<T, E>,
{
    /// Queues `events` and delivers whatever that makes deliverable.
    ///
    /// Returns whether the events were queued. They are rejected, and dropped outside the lock,
    /// once the delivery has stopped or the termination is already queued.
    pub fn send(&self, events: EventBatch<T, E>) -> bool {
        let action = self.0.with_mut(|state| state.enqueue_batch(events));
        self.perform(action)
    }

    /// Updates the resources and queues the events that update produced, under one lock.
    ///
    /// This is how a host changes what it owns, whether or not that emits anything: an update that
    /// emits nothing simply decides no events, and then no delivery can start here.
    ///
    /// `update` describes its outcome with an [`UpdateOutcome`]. It must not notify anyone or drop
    /// a value that can re-enter this delivery: it runs under the lock, so hand such a value to
    /// [`UpdateOutcome::with_drop_outside`] instead.
    ///
    /// Returns [`DeliveryStopped`], without running `update`, once the delivery has stopped.
    /// `update` and everything it captured are then dropped outside the lock.
    pub fn update<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(&mut R) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Result<Out, DeliveryStopped> {
        // Keep `update` out of the closure so that, when the delivery has stopped, its captures
        // are dropped only after the lock is released.
        let mut update = Some(update);
        let (action, drop_outside, result) = self
            .0
            .with_mut(|state| {
                let resources = state.resources_mut()?;
                let update = update.take().expect("the update runs at most once");
                let UpdateOutcome {
                    events,
                    drop_outside,
                    result,
                } = update(resources);
                let action = events.map(|events| state.enqueue_batch(events));
                Some((action, drop_outside, result))
            })
            .ok_or(DeliveryStopped)?;
        if let Some(action) = action {
            self.perform(action);
        }
        drop(drop_outside); // Drop after the delivery, outside the lock
        Ok(result)
    }

    /// Performs, with the lock released, what `enqueue_batch` deferred to outside it.
    fn perform(&self, action: EnqueueAction<T, E, OR>) -> bool {
        match action {
            EnqueueAction::Start {
                observer,
                first_next,
            } => {
                self.deliver(observer, first_next);
                true
            }
            EnqueueAction::Accepted => true,
            EnqueueAction::Rejected(events) => {
                drop(events); // Drop outside the lock to avoid potential deadlock
                false
            }
        }
    }

    /// Delivers `first_next` and then the queued events, one at a time.
    ///
    /// The lock is reacquired between two events, so an event that arrives during a delivery is
    /// delivered in arrival order, and a stop takes effect immediately — including between two
    /// values of one `EventBatch::NextBatch`: the loop then drops the observer instead of
    /// delivering to it.
    ///
    /// Every observer callback runs outside the lock. If one unwinds, the delivery is stopped, so
    /// a caught panic cannot leave it stuck in its delivering state — and locking from the guard is
    /// safe on the panicking thread for that same reason.
    fn deliver(&self, mut observer: OR, first_next: Option<T>) {
        if let Some(value) = first_next {
            let guard = on_panic(|| self.stop());
            observer.on_next(value);
            drop(guard);
        }

        loop {
            match self.0.with_mut(|state| state.next_step(observer)) {
                Step::Next(next_observer, value) => {
                    observer = next_observer;
                    let guard = on_panic(|| self.stop());
                    observer.on_next(value);
                    drop(guard);
                }
                Step::Terminate {
                    observer,
                    termination,
                    resources,
                } => {
                    let guard = on_panic(|| self.stop());
                    observer.on_termination(termination);
                    drop(guard);
                    // The resources outlive the terminal notification, so a host can dispose its
                    // source only after its downstream was told the stream ended. Unwinding from
                    // the callback drops them too.
                    drop(resources);
                    return;
                }
                Step::Parked => return,
                Step::Stopped(observer) => {
                    drop(observer); // Drop outside the lock to avoid potential deadlock
                    return;
                }
            }
        }
    }
}

impl<T, E, OR, R> WeakSerializedDelivery<T, E, OR, R> {
    pub fn upgrade(&self) -> Option<SerializedDelivery<T, E, OR, R>> {
        self.0.upgrade().map(SerializedDelivery)
    }
}

impl<T, E, OR, R> State<T, E, OR, R> {
    fn resources_mut(&mut self) -> Option<&mut R> {
        match self {
            Self::Idle { resources, .. } | Self::Delivering { resources, .. } => Some(resources),
            Self::Stopped => None,
        }
    }

    fn enqueue_batch(&mut self, events: EventBatch<T, E>) -> EnqueueAction<T, E, OR> {
        match self {
            // The delivery loop holds the observer and picks these events up on its own.
            Self::Delivering { pending, .. } => match pending.push_batch(events) {
                Some(rejected) => EnqueueAction::Rejected(rejected),
                None => EnqueueAction::Accepted,
            },
            Self::Stopped => EnqueueAction::Rejected(events),
            Self::Idle { .. } => {
                // The queue is built from the batch instead of being pushed to and popped from:
                // a fresh queue rejects nothing, and the first value is delivered directly, so it
                // never enters the queue and a single-value batch allocates no queue at all.
                let (first_next, pending) = PendingEvents::from_batch(events);
                if first_next.is_none() && pending.is_empty() {
                    // An empty `NextBatch` is a no-op, consistent with the delivering state.
                    return EnqueueAction::Accepted;
                }

                // The batch is known to need a delivery only now, so the observer is taken out of
                // the state only now. It is put back into `Delivering` right below, so nothing the
                // state owned is dropped under the lock.
                let Self::Idle {
                    observer,
                    resources,
                } = std::mem::replace(self, Self::Stopped)
                else {
                    unreachable!()
                };
                *self = Self::Delivering { pending, resources };
                EnqueueAction::Start {
                    observer,
                    first_next,
                }
            }
        }
    }

    /// Returns the next step of a running delivery loop, moving to `Stopped` and handing the
    /// resources back to the caller before a terminal event.
    fn next_step(&mut self, observer: OR) -> Step<T, E, OR, R> {
        match self {
            Self::Delivering { pending, .. } => {
                if let Some(value) = pending.pop_next() {
                    return Step::Next(observer, value);
                }
            }
            Self::Stopped => return Step::Stopped(observer),
            // The observer is out of the state only while a delivery is running.
            Self::Idle { .. } => unreachable!("a delivery loop only runs in the delivering state"),
        }

        // Everything the state owns is handed to the caller or put back into `Idle` below, so
        // nothing is dropped under the lock.
        let Self::Delivering {
            mut pending,
            resources,
        } = std::mem::replace(self, Self::Stopped)
        else {
            unreachable!()
        };
        match pending.take_termination() {
            Some(termination) => {
                // The terminal event is only popped after every value, so this queue is empty and
                // owns no user value while it is dropped here.
                drop(pending);
                Step::Terminate {
                    observer,
                    termination,
                    resources,
                }
            }
            None => {
                *self = Self::Idle {
                    observer,
                    resources,
                };
                Step::Parked
            }
        }
    }
}
