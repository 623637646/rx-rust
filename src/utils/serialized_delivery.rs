//! The state machine that serializes the events delivered to one observer, together with the lock
//! that guards it and the only operations allowed to drive it.
//!
//! [`SerializedDelivery`] is a shared handle. Its lock never leaves this module, so a host cannot
//! hold it across a notification, a drop, or a second lock: every transition, the delivery loop,
//! and the rule that nothing is dropped or notified under the lock live here.
//!
//! The observer lives apart from the rest of the state, which is what a [`DeliveryStop`] holds:
//! stopping a delivery from a disposal must not require naming the observer's type, since the
//! type of a disposal cannot depend on its observer (see
//! [`ObservableTypes`](crate::observable::ObservableTypes)). The rest of the state — the queue and
//! the host's resources — is guarded by the delivery's lock, and the observer cell is only reached
//! while that lock is held, so the two behave as one state.
//!
//! A host that must change its own data, and emit the resulting events atomically, does so through
//! [`SerializedDelivery::update`], which runs its callback under the same lock that then queues
//! the events. The callback describes its outcome with an [`UpdateOutcome`], the one way to hand
//! this module events to queue and a value to drop once they were delivered.
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observer::{callback_observer::CallbackObserver, EventBatch, Flow, Termination},
//!     thread_mode::Local,
//!     utils::serialized_delivery::{SerializedDelivery, UpdateOutcome},
//! };
//! use std::sync::{Arc, Mutex};
//!
//! let seen = Arc::new(Mutex::new(Vec::new()));
//! let seen_in_observer = Arc::clone(&seen);
//! let observer = CallbackObserver::new(move |value| seen_in_observer.lock().unwrap().push(value), |_| {});
//!
//! // The resources here are a counter the host updates under the delivery's lock.
//! let delivery = SerializedDelivery::<Local, i32, (), _, i32>::idle(observer, 0);
//! assert_eq!(delivery.send(EventBatch::Next(1)), Flow::Continue);
//! let count = delivery.update(|count| {
//!     *count += 1;
//!     UpdateOutcome::new(*count).with_next_event(10)
//! });
//! assert_eq!(count, Ok(1));
//! assert_eq!(delivery.send(EventBatch::Termination(Termination::Completed)), Flow::Stop);
//! assert_eq!(*seen.lock().unwrap(), [1, 10]);
//! ```

use crate::thread_mode::mutable::{MutableBoolHelper, MutableExt, MutableHelper};
use crate::{
    observer::{EventBatch, Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{on_panic::on_panic, pending_events::PendingEvents},
};
use educe::Educe;

/// A shared, serialized delivery of events to one observer.
///
/// `R` is whatever the host owns alongside the observer. It is dropped, outside the lock, once the
/// delivery stops — after the terminal notification when the delivery stops by terminating. The
/// observer is released before it wherever the stopping code owns the observer; a stop that finds
/// the observer held by a delivery loop elsewhere, a [`DeliveryStop`], or a callback that unwinds,
/// drops `R` first.
///
/// The pointers are the ones the thread mode `M` picks.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct SerializedDelivery<M: ThreadMode, T, E, OR, R> {
    // Declared first so that, when the last handle goes, the observer is released before the
    // resources, as a stop releases them.
    #[educe(Debug(ignore))]
    observer: M::Ptr<Option<OR>>,
    #[educe(Debug(ignore))]
    core: M::Ptr<Core<T, E, R>>,
    /// Set by a [`DeliveryStop`], the one stop that cannot reach the observer and may leave it
    /// parked: only then does the `Drop` of a handle have something to release, so the drop of a
    /// handle takes no lock before it.
    #[educe(Debug(ignore))]
    stopped_by_handle: M::Flag,
}

/// Stops a [`SerializedDelivery`] without naming its observer's type: what a disposal holds.
///
/// Stopping drops the queued events and the resources, outside the lock, and rejects every later
/// event. The observer is released by the first handle of the delivery that is dropped afterwards —
/// typically at once, since dropping the resources disposes the sources that hold those handles,
/// and even while another handle, such as a scheduler task's, lives on — or by the next event or
/// update that arrives, whichever comes first.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct DeliveryStop<M: ThreadMode, T, E, R> {
    #[educe(Debug(ignore))]
    core: M::Ptr<Core<T, E, R>>,
    #[educe(Debug(ignore))]
    stopped_by_handle: M::Flag,
}

/// Everything a delivery holds except its observer.
enum Core<T, E, R> {
    /// No delivery is running, and the observer is parked in the observer cell.
    Idle { resources: R },
    /// The observer is held by the delivery loop and its cell is empty, while re-entrant events
    /// wait here.
    Delivering {
        pending: PendingEvents<T, E>,
        resources: R,
    },
    /// The resources are gone. Every later event is rejected.
    ///
    /// The one state that does not say where the observer is: a [`DeliveryStop`] that stopped an
    /// idle delivery cannot reach the cell, so the observer may still be parked there. Whoever
    /// finds this state under the lock next takes it out — an event, an update, or a handle being
    /// dropped.
    Stopped,
}

/// The action to perform after releasing the lock used to call `enqueue_batch`.
enum EnqueueAction<T, E, OR> {
    /// Start a delivery loop with the observer taken out of its cell.
    Start { observer: OR, first_next: Option<T> },
    /// The batch was queued for a running delivery, or needed no work, and answers this flow:
    /// [`Flow::Stop`] exactly when it queued the termination.
    Queued(Flow),
    /// The delivery was already stopped or a termination was already queued. `observer` is the one
    /// a [`DeliveryStop`] left parked, taken out to be dropped too.
    Rejected {
        events: EventBatch<T, E>,
        observer: Option<OR>,
    },
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
    /// Nothing is queued; the observer was parked back into its cell.
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
    /// An outcome that gives `result` back to the caller of the update, with the events and the
    /// value to drop still to be decided.
    pub fn new(result: R) -> Self {
        Self {
            events: None,
            drop_outside: DropUndecided,
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
    /// Hands `drop_outside` over to be dropped after the lock is released — and after the events
    /// are delivered. This is where a value the update displaced, or rejected, goes.
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

    /// States that nothing needs to be dropped outside the lock.
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
        UpdateOutcome {
            events: Some(events),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    /// States that the update queues nothing.
    pub fn without_events(self) -> UpdateOutcome<T, E, R, DO, true> {
        UpdateOutcome {
            events: None,
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }
}

impl<M: ThreadMode, T, E, R> DeliveryStop<M, T, E, R> {
    /// Stops the delivery. Stopping again is a no-op.
    pub fn stop(&self) {
        // `Stopped` is the only variant that owns nothing, so replacing the state with it takes
        // the queued events and the resources out. Binding them here drops them outside the lock.
        // The flag is set under the lock, so a handle that reads it and then locks finds the state
        // stopped.
        let _deferred_drop = self.core.with_mut(|core| {
            self.stopped_by_handle.write(true);
            std::mem::replace(core, Core::Stopped)
        });
    }
}

impl<M: ThreadMode, T, E, OR, R> SerializedDelivery<M, T, E, OR, R> {
    /// Starts with the observer attached and parked, waiting for the first event.
    pub fn idle(observer: OR, resources: R) -> Self {
        Self {
            observer: M::ptr(Some(observer)),
            core: M::ptr(Core::Idle { resources }),
            stopped_by_handle: M::Flag::default(),
        }
    }

    /// Stops the delivery, dropping the observer without notifying it. Stopping again is a no-op.
    pub fn stop(&self) {
        // The observer parked in its cell is taken under the same lock that stops the state, so it
        // is this call that drops it, before what the state held: downstream is released before
        // its sources are, and anything its drop emits is rejected. A loop running elsewhere
        // holds the observer instead, leaving the cell empty, and drops it itself when it sees
        // the stop.
        let (parked, deferred_drop) = self.core.with_mut(|core| {
            (
                self.observer.take_value(),
                std::mem::replace(core, Core::Stopped),
            )
        });
        drop(parked); // Drop outside the lock to avoid potential deadlock
        drop(deferred_drop);
    }

    /// Creates the handle that stops this delivery without naming its observer.
    pub fn stop_handle(&self) -> DeliveryStop<M, T, E, R> {
        DeliveryStop {
            core: self.core.clone(),
            stopped_by_handle: self.stopped_by_handle.clone(),
        }
    }
}

impl<M: ThreadMode, T, E, OR, R> SerializedDelivery<M, T, E, OR, R>
where
    OR: Observer<T, E>,
{
    /// Queues `events` and delivers whatever became deliverable because of them.
    ///
    /// Returns whether the observer still accepts events. [`Flow::Stop`] means the delivery has
    /// stopped or a termination was already queued, so the events were rejected and dropped outside
    /// the lock; it also means `events` itself carried a termination, since nothing can be queued
    /// after it. A value queued behind a delivery running elsewhere is reported as
    /// [`Flow::Continue`]; see [`Flow`].
    pub fn send(&self, events: EventBatch<T, E>) -> Flow {
        let action = self
            .core
            .with_mut(|core| core.enqueue_batch(events, &self.observer));
        self.perform(action)
    }

    /// Updates the resources and queues the events that update produced, under one lock.
    ///
    /// Every change a host makes to its own data goes through here, whether it emits or not: an
    /// update that emits nothing simply decides no events, and then no delivery starts from here.
    ///
    /// `update` describes its outcome with an [`UpdateOutcome`]. It must **not notify anyone or
    /// drop anything that can re-enter this delivery** — it runs under the lock; hand such values
    /// to [`UpdateOutcome::with_drop_outside`] instead.
    ///
    /// If the delivery has stopped, `update` does not run, [`DeliveryStopped`] is returned, and
    /// `update` is dropped, with everything it captured, outside the lock.
    pub fn update<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(&mut R) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Result<Out, DeliveryStopped> {
        self.update_with_flow(update).map(|(result, _)| result)
    }

    /// [`update`](Self::update), also reporting whether the observer still accepts events.
    ///
    /// The flow is what delivering the queued events answered, and [`Flow::Continue`] when the
    /// update queued none. A delivery that has already stopped answers [`DeliveryStopped`] instead
    /// of a flow, which a host that only cares about the flow maps to [`Flow::Stop`].
    pub fn update_with_flow<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(&mut R) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Result<(Out, Flow), DeliveryStopped> {
        // Keep `update` out of the closure so that, when the delivery has stopped, its captures
        // are dropped only after the lock is released.
        let mut update = Some(update);
        let outcome = self.core.with_mut(|core| {
            let Some(resources) = core.resources_mut() else {
                // Stopped, so a `DeliveryStop` may have left the observer parked.
                return Err(self.observer.take_value());
            };
            let update = update.take().expect("the update runs at most once");
            let UpdateOutcome {
                events,
                drop_outside,
                result,
            } = update(resources);
            let action = match events {
                Some(events) => core.enqueue_batch(events, &self.observer),
                None => EnqueueAction::Queued(Flow::Continue),
            };
            Ok((action, drop_outside, result))
        });
        let (action, drop_outside, result) = match outcome {
            Ok(outcome) => outcome,
            Err(parked) => {
                drop(update); // Drop outside the lock
                drop(parked);
                return Err(DeliveryStopped);
            }
        };
        let flow = self.perform(action);
        drop(drop_outside); // Drop after the delivery, outside the lock
        Ok((result, flow))
    }

    /// Carries out, with the lock released, what `enqueue_batch` decided under it.
    ///
    /// A batch that carries a termination answers [`Flow::Stop`] on every path: a delivery it
    /// starts ends by terminating, one it is queued behind answers `Queued(Flow::Stop)`, and a
    /// rejection is a stop.
    fn perform(&self, action: EnqueueAction<T, E, OR>) -> Flow {
        match action {
            EnqueueAction::Start {
                observer,
                first_next,
            } => self.deliver(observer, first_next),
            EnqueueAction::Queued(flow) => flow,
            EnqueueAction::Rejected { events, observer } => {
                drop(events); // Drop outside the lock to avoid potential deadlock
                drop(observer);
                Flow::Stop
            }
        }
    }

    /// Delivers `next`, then every queued event, one at a time.
    ///
    /// The lock is taken again between two events, so events arriving during the delivery are
    /// delivered in arrival order, and a stop takes effect immediately — even between two values
    /// of an `EventBatch::NextBatch`: the loop drops the observer instead of feeding it further.
    ///
    /// Every observer callback runs outside the lock. An `on_next` that unwinds stops the delivery,
    /// so a caught panic does not leave it stuck in the delivering state — and taking the lock
    /// from the guard is safe on the panicking thread for that very reason. `on_termination` needs
    /// no guard: the state is already stopped when it runs, and the unwind drops the resources.
    ///
    /// Returns [`Flow::Stop`] when this delivery ended — the observer stopped, it was terminated,
    /// or the delivery had stopped already — and [`Flow::Continue`] when the observer was parked
    /// back for the next event.
    fn deliver(&self, mut observer: OR, mut next: Option<T>) -> Flow {
        loop {
            if let Some(value) = next.take() {
                let guard = on_panic(|| self.stop());
                let flow = observer.on_next(value);
                drop(guard);
                if flow.is_stop() {
                    // It has ended its own stream or been disposed, so it is released like a
                    // disposed observer instead of being terminated. The loop holds it, so its
                    // cell is empty; it is dropped as `stop` drops it, after the state is stopped
                    // and before what the state held.
                    let deferred_drop = self.core.replace_value(Core::Stopped);
                    drop(observer); // Drop outside the lock to avoid potential deadlock
                    drop(deferred_drop);
                    return Flow::Stop;
                }
            }

            match self
                .core
                .with_mut(|core| core.next_step(observer, &self.observer))
            {
                Step::Next(next_observer, value) => {
                    observer = next_observer;
                    next = Some(value);
                }
                Step::Terminate {
                    observer,
                    termination,
                    resources,
                } => {
                    observer.on_termination(termination);
                    // The resources outlive the terminal notification, so a host can dispose its
                    // source only after its downstream was told the stream ended.
                    drop(resources);
                    return Flow::Stop;
                }
                Step::Parked => return Flow::Continue,
                Step::Stopped(observer) => {
                    drop(observer); // Drop outside the lock to avoid potential deadlock
                    return Flow::Stop;
                }
            }
        }
    }
}

impl<M: ThreadMode, T, E, OR, R> Drop for SerializedDelivery<M, T, E, OR, R> {
    /// Releases an observer that a [`DeliveryStop`] left parked.
    ///
    /// A host whose handles are all held by its sources gets this anyway, from the last one. One
    /// that a scheduler task also holds gets it from the first handle dropped after the stop: the
    /// handle of the source, as the source is disposed, so the observer is released before
    /// `dispose` returns rather than when the runtime gets round to dropping the cancelled task.
    ///
    /// Only a [`DeliveryStop`] leaves the observer parked in a stopped delivery, so until one has
    /// stopped it, which its flag says without a lock, there is nothing to release and no lock is
    /// taken. A handle can therefore be dropped anywhere, even under the delivery's own lock, as
    /// before this release existed: after the stop, nothing runs under that lock that could drop
    /// one.
    fn drop(&mut self) {
        if !self.stopped_by_handle.read() {
            return;
        }
        let parked = self.core.with_mut(|core| match core {
            Core::Stopped => self.observer.take_value(),
            Core::Idle { .. } | Core::Delivering { .. } => None,
        });
        drop(parked); // Drop outside the lock to avoid potential deadlock
    }
}

impl<T, E, R> Core<T, E, R> {
    fn resources_mut(&mut self) -> Option<&mut R> {
        match self {
            Self::Idle { resources } | Self::Delivering { resources, .. } => Some(resources),
            Self::Stopped => None,
        }
    }

    /// Queues `events`, taking the observer out of `observer_cell` — whose lock nests inside this
    /// one, and only ever in this order — when a delivery has to start.
    fn enqueue_batch<OR, P>(
        &mut self,
        events: EventBatch<T, E>,
        observer_cell: &P,
    ) -> EnqueueAction<T, E, OR>
    where
        P: MutableHelper<Value = Option<OR>>,
    {
        match self {
            // The delivery loop holds the observer and picks these events up on its own.
            // Nothing could be queued after a termination, so a queue that accepted the batch and
            // is now terminated got its termination from this very batch.
            Self::Delivering { pending, .. } => match pending.push_batch(events) {
                Some(events) => EnqueueAction::Rejected {
                    events,
                    observer: None,
                },
                None if pending.is_terminated() => EnqueueAction::Queued(Flow::Stop),
                None => EnqueueAction::Queued(Flow::Continue),
            },
            // A `DeliveryStop` may have left the observer parked.
            Self::Stopped => EnqueueAction::Rejected {
                events,
                observer: observer_cell.take_value(),
            },
            Self::Idle { .. } => {
                // The queue is built from the batch instead of being pushed to and popped from:
                // a fresh queue rejects nothing, and the first value is delivered directly, so it
                // never enters the queue and a single-value batch allocates no queue at all.
                let (first_next, pending) = PendingEvents::from_batch(events);
                if first_next.is_none() && pending.is_empty() {
                    // An empty `NextBatch` is a no-op, consistent with the delivering state.
                    return EnqueueAction::Queued(Flow::Continue);
                }

                // The batch is known to need a delivery only now, so the observer is taken out of
                // its cell only now. Nothing the state owned is dropped under the lock.
                let observer = observer_cell
                    .take_value()
                    .expect("an idle delivery has its observer parked");
                let Self::Idle { resources } = std::mem::replace(self, Self::Stopped) else {
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
    fn next_step<OR, P>(&mut self, observer: OR, observer_cell: &P) -> Step<T, E, OR, R>
    where
        P: MutableHelper<Value = Option<OR>>,
    {
        match self {
            Self::Delivering { pending, .. } => {
                if let Some(value) = pending.pop_next() {
                    return Step::Next(observer, value);
                }
            }
            Self::Stopped => return Step::Stopped(observer),
            // The observer is out of its cell only while a delivery is running.
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
                // The observer goes back into its cell before the state turns idle, under this
                // lock, so whoever finds the state idle finds the observer parked.
                let previous = observer_cell.replace_value(Some(observer));
                debug_assert!(previous.is_none(), "a running delivery owns the observer");
                *self = Self::Idle { resources };
                Step::Parked
            }
        }
    }
}
