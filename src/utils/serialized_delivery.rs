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
//! this module events to queue and a value to drop after sending or queueing them, and the update
//! answers with an [`Updated`]: the callback's result, and whether the observer still accepts
//! events. Both live in a module of their own, which keeps the fields of an outcome out of reach
//! of the delivery.
//!
//! [`UpdateOutcome`] and [`Updated`] are what a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) takes and
//! answers too, so they are public, and re-exported by
//! [`subscribe_with_context`](crate::utils::subscribe_with_context); the delivery itself is not.

use crate::{
    observer::{EventBatch, Flow, Observer, Termination},
    thread_mode::{
        ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
    utils::{on_panic::on_panic, pending_events::PendingEvents},
};
use educe::Educe;
use locked_core::LockedCore;

mod update_outcome;

pub use update_outcome::{DropDecided, DropUndecided, UpdateOutcome, Updated};

/// A shared, serialized delivery of events to one observer.
///
/// `R` is whatever the host owns alongside the observer, its sources' subscriptions typically. It
/// is dropped, outside the lock, once the delivery stops — after the terminal notification when the
/// delivery stops by terminating. Otherwise the observer is released without a notification, and
/// only after `R` is dropped, so that its release tells downstream that its sources are gone. The
/// exceptions are the stops that cannot reach the observer: a [`DeliveryStop`] leaves it parked
/// for the next event, update or handle dropped — typically the handle of a source, as `R`
/// disposes it — and a stop that finds it held by a delivery loop on another thread leaves that
/// loop to release it, which it may do while `R` is still being dropped.
///
/// The pointers are the ones the thread mode `M` picks.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub(crate) struct SerializedDelivery<M: ThreadMode, T, E, OR, R> {
    #[educe(Debug(ignore))]
    core: LockedCore<M, T, E, R>,
    // Declared after the core so that, when the last handle goes, the observer is released after
    // the resources, as a stop releases them.
    #[educe(Debug(ignore))]
    observer: M::Ptr<Option<OR>>,
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
pub(crate) struct DeliveryStop<M: ThreadMode, T, E, R> {
    #[educe(Debug(ignore))]
    core: LockedCore<M, T, E, R>,
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

mod locked_core {
    use super::Core;
    use crate::thread_mode::{
        ThreadMode,
        mutable::{MutableBoolHelper, MutableHelper},
    };
    use educe::Educe;

    /// The lock around a delivery's [`Core`], shared by every handle of the delivery and by its
    /// [`DeliveryStop`](super::DeliveryStop)s, together with a flag saying, without the lock,
    /// whether a [`stop_with`](Self::stop_with) has stopped it.
    ///
    /// Until a stop has run, nothing can be parked in a stopped core, so the `Drop` of a handle has
    /// nothing to release and, through [`after_stop`](Self::after_stop), takes no lock. The flag
    /// is raised under the lock, together with the stop, so a handle that reads it raised and then
    /// locks finds the state stopped. Its own module keeps the flag out of reach of the rest of the
    /// delivery, so that nothing else can raise it.
    #[derive(Educe)]
    #[educe(Clone(bound()))]
    pub(super) struct LockedCore<M: ThreadMode, T, E, R> {
        core: M::Ptr<Core<T, E, R>>,
        stopped: M::Flag,
    }

    impl<M: ThreadMode, T, E, R> LockedCore<M, T, E, R> {
        pub(super) fn new(core: Core<T, E, R>) -> Self {
            Self {
                core: M::ptr(core),
                stopped: M::Flag::default(),
            }
        }

        /// Runs `callback` on the core, under the lock.
        pub(super) fn with_mut<Out>(
            &self,
            callback: impl FnOnce(&mut Core<T, E, R>) -> Out,
        ) -> Out {
            self.core.with_mut(callback)
        }

        /// Stops the core and runs `callback` under the same lock: returns what the core held and
        /// what `callback` returned, to be dropped outside the lock, in that order.
        ///
        /// Every stop goes through here, whether or not it can reach the observer: a
        /// `DeliveryStop` passes a callback that does nothing, the delivery one that takes the
        /// observer out of its cell. Only a delivery that terminates ends elsewhere, in its loop,
        /// which consumes the observer.
        pub(super) fn stop_with<Out>(
            &self,
            callback: impl FnOnce() -> Out,
        ) -> (Core<T, E, R>, Out) {
            self.core.with_mut(|core| {
                self.stopped.write(true);
                (std::mem::replace(core, Core::Stopped), callback())
            })
        }

        /// Runs `callback` under the lock once a [`stop_with`](Self::stop_with) has run, and takes
        /// no lock before: [`None`] until then. The core is then stopped for good.
        pub(super) fn after_stop<Out>(&self, callback: impl FnOnce() -> Out) -> Option<Out> {
            if !self.stopped.read() {
                return None;
            }
            Some(self.core.with_mut(|core| {
                debug_assert!(matches!(core, Core::Stopped), "a stop is final");
                callback()
            }))
        }
    }
}

/// The action to perform after releasing the lock used to call `enqueue_batch`.
enum EnqueueAction<T, E, OR> {
    /// Start a delivery loop with the observer taken out of its cell.
    Start { observer: OR, first_next: Option<T> },
    /// The batch was queued for a running delivery, or needed no work, and answers this flow:
    /// [`Flow::Stop`] exactly when it queued the termination, or, for an update that queued
    /// nothing, when one was queued already.
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

impl<M: ThreadMode, T, E, R> DeliveryStop<M, T, E, R> {
    /// Stops the delivery. Stopping again is a no-op.
    pub(crate) fn stop(&self) {
        // What the state held, the queued events and the resources, is bound here and so dropped
        // outside the lock. The observer's type is not known here, so an observer parked in its
        // cell stays there for whoever finds the stop next.
        let (_deferred_drop, ()) = self.core.stop_with(|| ());
    }
}

impl<M: ThreadMode, T, E, OR, R> SerializedDelivery<M, T, E, OR, R> {
    /// Starts with the observer attached and parked, waiting for the first event.
    pub(crate) fn idle(observer: OR, resources: R) -> Self {
        Self {
            observer: M::ptr(Some(observer)),
            core: LockedCore::new(Core::Idle { resources }),
        }
    }

    /// Stops the delivery, dropping the observer without notifying it. Stopping again is a no-op.
    fn stop(&self) {
        // The observer parked in its cell is taken under the same lock that stops the state, so it
        // is this call that drops it, after what the state held: the sources are released before
        // downstream is, and anything either drop emits is rejected. A loop running elsewhere
        // holds the observer instead, leaving the cell empty, and drops it itself when it sees
        // the stop.
        let (deferred_drop, parked) = self.core.stop_with(|| self.observer.take_value());
        // Outside the lock to avoid potential deadlock. Should the first drop unwind, the
        // observer is still released, after it, by the unwind.
        drop(deferred_drop);
        drop(parked);
    }

    /// Creates the handle that stops this delivery without naming its observer.
    pub(crate) fn stop_handle(&self) -> DeliveryStop<M, T, E, R> {
        DeliveryStop {
            core: self.core.clone(),
        }
    }

    /// Whether the delivery still accepts events: it has not stopped, and no termination is
    /// queued behind a delivery running elsewhere.
    ///
    /// A stop concurrent with this call may land right after it, so a host uses this to skip work
    /// a stream that is already over has no use for, never as a guarantee that the work it then
    /// starts outlives the check.
    pub(crate) fn is_active(&self) -> bool {
        self.core.with_mut(|core| core.is_active())
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
    pub(crate) fn send(&self, events: EventBatch<T, E>) -> Flow {
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
    /// Answers the result of `update` and the flow, as [`Updated::Ran`] describes them. If the
    /// delivery has stopped, `update` does not run, [`Updated::Stopped`] is answered, and `update`
    /// is dropped, with everything it captured, outside the lock.
    pub(crate) fn update<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(&mut R) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Updated<Out> {
        // Keep `update` out of the closure so that, when the delivery has stopped, its captures
        // are dropped only after the lock is released.
        let mut update = Some(update);
        let outcome = self.core.with_mut(|core| {
            let Some(resources) = core.resources_mut() else {
                // Stopped, so a `DeliveryStop` may have left the observer parked.
                return Err(self.observer.take_value());
            };
            let update = update.take().expect("the update runs at most once");
            let (events, drop_outside, flow, result) = update(resources).into_parts();
            let action = match events {
                Some(events) => core.enqueue_batch(events, &self.observer),
                None if core.is_active() => EnqueueAction::Queued(Flow::Continue),
                None => EnqueueAction::Queued(Flow::Stop),
            };
            Ok((action, drop_outside, flow, result))
        });
        let (action, drop_outside, answered, result) = match outcome {
            Ok(outcome) => outcome,
            Err(parked) => {
                drop(update); // Drop outside the lock
                drop(parked);
                return Updated::Stopped;
            }
        };
        let delivered = self.perform(action);
        drop(drop_outside); // Drop after sending or queueing the events, outside the lock.
        let flow = if answered.is_stop() {
            Flow::Stop
        } else {
            delivered
        };
        Updated::Ran { result, flow }
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
                    // disposed observer instead of being terminated. The loop holds it, so `stop`
                    // finds its cell empty and drops only what the state held; the observer is
                    // dropped right after, in the order `stop` drops a parked one.
                    self.stop();
                    drop(observer);
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
    /// Only a [`DeliveryStop`] leaves the observer parked in a stopped delivery, so until a stop
    /// has run, which a flag of the [`LockedCore`] says without a lock, there is nothing to release
    /// and no lock is taken. A handle can therefore be dropped anywhere, even under the delivery's
    /// own lock: after the stop, nothing runs under that lock that could drop one.
    fn drop(&mut self) {
        let parked = self
            .core
            .after_stop(|| self.observer.take_value())
            .flatten();
        drop(parked); // Drop outside the lock to avoid potential deadlock
    }
}

impl<T, E, R> Core<T, E, R> {
    /// Whether events are still accepted: the delivery has not stopped, and no termination is
    /// queued behind a delivery running elsewhere.
    fn is_active(&self) -> bool {
        match self {
            Self::Idle { .. } => true,
            Self::Delivering { pending, .. } => !pending.is_terminated(),
            Self::Stopped => false,
        }
    }

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

#[cfg(test)]
mod tests {
    // Tests of the delivery on its own, without any operator around it.
    //
    // They drive the state machine directly, so they can check what only this module promises: the
    // order in which queued and re-entrant events reach the observer, when the observer and the
    // resources are dropped, and that nothing is ever dropped nor notified while the state is
    // locked.
    //
    // The last one is checked with values whose `Drop` re-enters the delivery: a drop that ran
    // under the lock would deadlock, or fail the re-entry check of debug builds, instead of
    // recording its note.

    use super::{SerializedDelivery, UpdateOutcome, Updated};
    use crate::{
        observer::{EventBatch, Flow, Observer, Termination, boxed_observer::SendBoxedObserver},
        thread_mode::{
            Shared,
            mutable::{MutableBoolHelper, MutableExt, MutableHelper},
        },
    };
    #[cfg(panic = "unwind")]
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::sync::{Arc, Mutex, atomic::AtomicBool};

    type TestError = &'static str;
    const ERROR: TestError = "boom";

    type TestObserver = SendBoxedObserver<'static, Value, TestError>;
    type TestDelivery = SerializedDelivery<Shared, Value, TestError, TestObserver, Resources>;
    type TestOutcome<R> = UpdateOutcome<Value, TestError, R>;

    // MARK: - Observing drops

    /// A callback run while a [`DropProbe`] is being dropped.
    type DropCallback = Box<dyn FnOnce() + Send>;

    /// A value that runs a callback while it is being dropped, which is how a test observes the
    /// drop of what embeds it, and re-enters the delivery from that drop.
    struct DropProbe(Option<DropCallback>);

    impl DropProbe {
        fn new() -> Self {
            Self(None)
        }

        /// Runs `callback` when this probe is dropped, after any callback it already runs.
        fn on_drop(mut self, callback: DropCallback) -> Self {
            self.also_on_drop(callback);
            self
        }

        /// Adds `callback` to a probe already owned by a value.
        fn also_on_drop(&mut self, callback: DropCallback) {
            self.0 = Some(match self.0.take() {
                Some(existing) => Box::new(move || {
                    existing();
                    callback();
                }),
                None => callback,
            });
        }
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            if let Some(callback) = self.0.take() {
                callback();
            }
        }
    }

    // MARK: - The log the tests assert on

    /// One thing that happened to the observer, to the resources, or to a value.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Record {
        Next(i32),
        Termination(Termination<TestError>),
        ObserverDropped,
        ResourcesDropped,
        /// Written by a [`DropCallback`], which is how a test observes the drop of a value.
        Note(&'static str),
    }

    type Log = Arc<Mutex<Vec<Record>>>;

    fn new_log() -> Log {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn record(log: &Log, record: Record) {
        log.with_mut(|values| values.push(record));
    }

    fn records(log: &Log) -> Vec<Record> {
        log.clone_value()
    }

    /// The values that reached the observer, which is what most tests assert on.
    fn values(log: &Log) -> Vec<i32> {
        records(log)
            .into_iter()
            .filter_map(|record| match record {
                Record::Next(value) => Some(value),
                _ => None,
            })
            .collect()
    }

    /// A callback that records `record_value` when whatever owns it is dropped.
    fn record_on_drop(log: &Log, record_value: Record) -> DropCallback {
        let log = log.clone();
        Box::new(move || record(&log, record_value))
    }

    /// A callback that only notes that whatever owned it was dropped.
    fn note(log: &Log, text: &'static str) -> DropCallback {
        record_on_drop(log, Record::Note(text))
    }

    /// A callback that notes the drop and then re-enters the delivery, which only works when the
    /// drop runs with the state unlocked.
    ///
    /// The value it sends carries no callback of its own, so re-entering never recurses any
    /// further.
    fn note_and_reenter(handle: &DeliveryHandle, log: &Log, text: &'static str) -> DropCallback {
        let handle = handle.clone();
        let log = log.clone();
        Box::new(move || {
            record(&log, Record::Note(text));
            // Locking here would deadlock, or fail the re-entry check of debug builds, if this drop
            // ran while the state was locked.
            let _ = handle.get().send(next(99));
        })
    }

    // MARK: - What the delivery carries

    /// A value of the tested stream, numbered so that the tests can tell one from another.
    ///
    /// The probe it embeds is what reports the drop. A value doubles as the probe handed to
    /// [`UpdateOutcome::with_drop_outside`] and as the capture of an update that never runs, so
    /// that every drop this module promises is observed the same way.
    struct Value {
        number: i32,
        probe: DropProbe,
    }

    impl Value {
        fn new(number: i32) -> Self {
            Self {
                number,
                probe: DropProbe::new(),
            }
        }

        fn on_drop(mut self, callback: DropCallback) -> Self {
            self.probe = self.probe.on_drop(callback);
            self
        }
    }

    /// What the host owns alongside the observer: a model, and a probe reporting its drop.
    struct Resources {
        model: i32,
        probe: DropProbe,
    }

    impl Resources {
        fn new(model: i32, log: &Log) -> Self {
            Self {
                model,
                probe: DropProbe::new().on_drop(record_on_drop(log, Record::ResourcesDropped)),
            }
        }

        /// Runs `callback` once the drop of the resources was recorded.
        fn on_drop(&mut self, callback: DropCallback) {
            self.probe.also_on_drop(callback);
        }
    }

    // MARK: - Reaching the delivery from what it owns

    /// A handle a test can hand to a callback before the delivery it points at exists, filled in
    /// once it was built.
    ///
    /// It holds the delivery strongly. Held by the observer, or by a value or the resources, it
    /// forms a cycle through the delivery that owns them, which is broken when the delivery lets go
    /// of what holds it: a stop, a termination, a rejected or delivered value — or, for the
    /// observer's, by [`BuiltDelivery`] when the test lets go of the delivery. The observer holds
    /// one only when the test asks for it, through a hook or [`Builder::handle`].
    #[derive(Clone)]
    struct DeliveryHandle(Arc<Mutex<Option<TestDelivery>>>);

    impl DeliveryHandle {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(None)))
        }

        fn of(delivery: &TestDelivery) -> Self {
            let handle = Self::new();
            handle.install(delivery);
            handle
        }

        fn install(&self, delivery: &TestDelivery) {
            let previous = self.0.replace_value(Some(delivery.clone()));
            assert!(previous.is_none(), "a handle is installed once");
        }

        fn get(&self) -> TestDelivery {
            self.try_get()
                .expect("the delivery is installed before anything can reach it")
        }

        fn try_get(&self) -> Option<TestDelivery> {
            self.0.clone_value()
        }

        /// Lets the delivery go, which breaks the cycle through whatever owns this handle.
        fn release(&self) {
            let delivery = self.0.take_value();
            drop(delivery); // Drop outside the lock
        }
    }

    // MARK: - The observer and how a test builds one

    /// An observer that records every event and every drop, and runs the hooks of its test.
    ///
    /// Each hook receives the delivery it is running in, so it can send, update, or stop from
    /// inside a notification, which is what re-entrancy looks like to this module.
    struct RecordingObserver<FN, FT> {
        /// Never read: it is here for its drop, and declared first so that the drop of the observer
        /// is recorded before the hooks it owns are dropped, which is the order a hand-written
        /// `Drop` gave.
        _probe: DropProbe,
        log: Log,
        handle: DeliveryHandle,
        on_next: FN,
        on_termination: Option<FT>,
        /// How many values this observer accepts before it answers [`Flow::Stop`], if it ever does.
        stop_after: Option<usize>,
        received: usize,
    }

    impl<FN, FT> Observer<Value, TestError> for RecordingObserver<FN, FT>
    where
        FN: FnMut(&TestDelivery, i32),
        FT: FnOnce(&TestDelivery, &Termination<TestError>),
    {
        fn on_next(&mut self, value: Value) -> Flow {
            let number = value.number;
            record(&self.log, Record::Next(number));
            if let Some(delivery) = self.handle.try_get() {
                (self.on_next)(&delivery, number);
            }
            self.received += 1;
            match self.stop_after {
                Some(stop_after) if self.received >= stop_after => Flow::Stop,
                _ => Flow::Continue,
            }
        }

        fn on_termination(mut self, termination: Termination<TestError>) {
            record(&self.log, Record::Termination(termination.clone()));
            let hook = self
                .on_termination
                .take()
                .expect("an observer is terminated at most once");
            if let Some(delivery) = self.handle.try_get() {
                hook(&delivery, &termination);
            }
            // `self` is dropped here, which records the drop of the observer after its termination.
        }
    }

    type NoNextHook = fn(&TestDelivery, i32);
    type NoTerminationHook = fn(&TestDelivery, &Termination<TestError>);

    /// Builds an idle delivery recording into `log`, with no hook and a model of `0`.
    fn builder(log: &Log) -> Builder<NoNextHook, NoTerminationHook> {
        Builder {
            log: log.clone(),
            handle: DeliveryHandle::new(),
            reaches_delivery: false,
            model: 0,
            on_next: |_delivery, _number| {},
            on_termination: |_delivery, _termination| {},
            on_observer_drop: None,
            stop_after: None,
        }
    }

    struct Builder<FN, FT> {
        log: Log,
        handle: DeliveryHandle,
        /// Whether the observer gets the handle: only when a hook, or a callback made from
        /// [`Self::handle`], has to reach the delivery.
        reaches_delivery: bool,
        model: i32,
        on_next: FN,
        on_termination: FT,
        on_observer_drop: Option<DropCallback>,
        stop_after: Option<usize>,
    }

    impl<FN, FT> Builder<FN, FT> {
        /// The handle of the delivery to be built, for a callback that has to be made before it.
        fn handle(&mut self) -> DeliveryHandle {
            self.reaches_delivery = true;
            self.handle.clone()
        }

        fn model(mut self, model: i32) -> Self {
            self.model = model;
            self
        }

        fn on_next<F>(self, on_next: F) -> Builder<F, FT>
        where
            F: FnMut(&TestDelivery, i32),
        {
            Builder {
                log: self.log,
                handle: self.handle,
                reaches_delivery: true,
                model: self.model,
                on_next,
                on_termination: self.on_termination,
                on_observer_drop: self.on_observer_drop,
                stop_after: self.stop_after,
            }
        }

        /// Makes the observer answer [`Flow::Stop`] once it has received `count` values.
        fn stop_after(mut self, count: usize) -> Self {
            self.stop_after = Some(count);
            self
        }

        fn on_termination<F>(self, on_termination: F) -> Builder<FN, F>
        where
            F: FnOnce(&TestDelivery, &Termination<TestError>),
        {
            Builder {
                log: self.log,
                handle: self.handle,
                reaches_delivery: true,
                model: self.model,
                on_next: self.on_next,
                on_termination,
                on_observer_drop: self.on_observer_drop,
                stop_after: self.stop_after,
            }
        }

        fn on_observer_drop(mut self, callback: DropCallback) -> Self {
            self.on_observer_drop = Some(callback);
            self
        }

        fn build(self) -> BuiltDelivery
        where
            FN: FnMut(&TestDelivery, i32) + Send + 'static,
            FT: FnOnce(&TestDelivery, &Termination<TestError>) + Send + 'static,
        {
            let handle = self.handle.clone();
            let mut probe =
                DropProbe::new().on_drop(record_on_drop(&self.log, Record::ObserverDropped));
            if let Some(callback) = self.on_observer_drop {
                probe.also_on_drop(callback);
            }
            let observer: TestObserver = SendBoxedObserver::new(RecordingObserver {
                _probe: probe,
                log: self.log.clone(),
                handle: self.handle,
                on_next: self.on_next,
                on_termination: Some(self.on_termination),
                stop_after: self.stop_after,
                received: 0,
            });
            let resources = Resources::new(self.model, &self.log);
            let delivery = SerializedDelivery::idle(observer, resources);
            if self.reaches_delivery {
                handle.install(&delivery);
            }
            BuiltDelivery { delivery, handle }
        }
    }

    /// The handle a test gets from [`Builder::build`]: a [`TestDelivery`] that, when it goes, lets
    /// go of the one the observer holds too.
    ///
    /// That breaks the cycle through the observer when the test lets go of the delivery, whether or
    /// not anything stopped it, so that a test needs no [`DeliveryHandle::release`] of its own, and
    /// the observer is still dropped with the last handle the test holds.
    struct BuiltDelivery {
        delivery: TestDelivery,
        handle: DeliveryHandle,
    }

    impl std::ops::Deref for BuiltDelivery {
        type Target = TestDelivery;

        fn deref(&self) -> &TestDelivery {
            &self.delivery
        }
    }

    impl Drop for BuiltDelivery {
        fn drop(&mut self) {
            self.handle.release();
        }
    }

    // MARK: - Shorthands for the batches the tests send

    fn next(number: i32) -> EventBatch<Value, TestError> {
        EventBatch::Next(Value::new(number))
    }

    fn next_batch(numbers: impl IntoIterator<Item = i32>) -> EventBatch<Value, TestError> {
        EventBatch::NextBatch(numbers.into_iter().map(Value::new).collect())
    }

    fn completed() -> EventBatch<Value, TestError> {
        EventBatch::Termination(Termination::Completed)
    }

    fn errored() -> EventBatch<Value, TestError> {
        EventBatch::Termination(Termination::Error(ERROR))
    }

    fn next_batch_and_completed(
        numbers: impl IntoIterator<Item = i32>,
    ) -> EventBatch<Value, TestError> {
        EventBatch::NextBatchAndTermination(
            numbers.into_iter().map(Value::new).collect(),
            Termination::Completed,
        )
    }

    // MARK: - Delivering events

    #[test]
    fn test_send_next_delivers_to_the_parked_observer() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(next(1)).is_continue());
        assert_eq!(records(&log), [Record::Next(1)]);

        // The observer is parked back after the delivery, so the next value starts another one.
        assert!(delivery.send(next(2)).is_continue());
        assert_eq!(values(&log), [1, 2]);
    }

    #[test]
    fn test_send_delivers_a_batch_in_order() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_continue());
        assert_eq!(values(&log), [1, 2, 3]);
    }

    #[test]
    fn test_a_termination_is_notified_before_the_resources_are_dropped() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(completed()).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_a_batch_delivers_its_values_before_its_error_termination() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(
            delivery
                .send(EventBatch::NextBatchAndTermination(
                    vec![Value::new(1), Value::new(2)],
                    Termination::Error(ERROR),
                ))
                .is_stop()
        );
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Next(2),
                Record::Termination(Termination::Error(ERROR)),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_a_next_and_termination_batch_delivers_its_value_first() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(
            delivery
                .send(EventBatch::NextAndTermination(
                    Value::new(1),
                    Termination::Completed,
                ))
                .is_stop()
        );
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_an_empty_batch_is_accepted_and_leaves_the_delivery_idle() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(EventBatch::NextBatch(vec![])).is_continue());
        assert_eq!(records(&log), []);

        // The observer was never taken out of the state, so it is still there for the next value.
        assert!(delivery.send(next(1)).is_continue());
        assert_eq!(values(&log), [1]);
    }

    #[test]
    fn test_an_empty_batch_sent_from_on_next_is_accepted_as_well() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    assert!(delivery.send(EventBatch::NextBatch(vec![])).is_continue());
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2])).is_continue());
        assert_eq!(values(&log), [1, 2]);
    }

    // MARK: - Rejecting events

    #[test]
    fn test_events_sent_after_a_termination_are_rejected() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(completed()).is_stop());
        assert!(delivery.send(next(1)).is_stop());
        assert!(delivery.send(errored()).is_stop());
        assert!(delivery.send(EventBatch::NextBatch(vec![])).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_events_sent_after_stop_are_rejected() {
        let log = new_log();
        let delivery = builder(&log).build();

        delivery.stop();
        assert!(delivery.send(next(1)).is_stop());
        assert!(delivery.send(completed()).is_stop());
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );
    }

    #[test]
    fn test_rejected_events_are_dropped_outside_the_lock() {
        let log = new_log();
        let delivery = builder(&log).build();
        assert!(delivery.send(completed()).is_stop());

        let rejected = Value::new(1).on_drop(note_and_reenter(
            &DeliveryHandle::of(&delivery),
            &log,
            "rejected dropped",
        ));
        assert!(delivery.send(EventBatch::Next(rejected)).is_stop());

        // The note proves the rejected value was dropped, and the delivery it re-entered from that
        // drop rejected what it sent instead of deadlocking on the lock of `send`.
        assert_eq!(
            records(&log)[3..],
            [Record::Note("rejected dropped")],
            "the rejected value must be dropped, outside the lock"
        );
    }

    #[test]
    fn test_events_sent_from_on_termination_are_rejected() {
        let log = new_log();
        let delivery = builder(&log)
            .on_termination(|delivery, _termination| {
                // The state is stopped before the terminal notification, so nothing can follow it.
                assert!(delivery.send(next(1)).is_stop());
                assert_eq!(
                    delivery.update(|_resources| UpdateOutcome::empty()),
                    Updated::Stopped
                );
            })
            .build();

        assert!(delivery.send(completed()).is_stop());
        assert_eq!(values(&log), []);
    }

    // MARK: - Stopping

    #[test]
    fn test_stop_drops_the_observer_without_notifying_it_and_is_idempotent() {
        let log = new_log();
        let delivery = builder(&log).build();

        delivery.stop();
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );

        delivery.stop();
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped],
            "stopping again must drop nothing more"
        );
    }

    #[test]
    fn test_stop_drops_the_observer_and_the_resources_outside_the_lock() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();
        delivery
            .update(|resources| {
                resources.on_drop(note_and_reenter(&handle, &log, "resources dropped"));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");

        delivery.stop();

        // Both drops re-entered the delivery, so neither of them ran while the state was locked.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::Note("resources dropped"),
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );
    }

    #[test]
    fn test_stop_from_on_next_drops_the_values_that_are_still_queued() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 2 {
                    delivery.stop();
                }
            })
            .build();

        assert!(
            delivery
                .send(EventBatch::NextBatch(vec![
                    Value::new(1),
                    Value::new(2),
                    Value::new(3).on_drop(note(&log, "3 dropped")),
                ]))
                .is_stop()
        );

        assert_eq!(values(&log), [1, 2]);
        assert_eq!(
            records(&log)[2..],
            [
                Record::Note("3 dropped"),
                Record::ResourcesDropped,
                // The loop drops the observer it holds only once it sees the stopped state.
                Record::ObserverDropped,
            ]
        );
        assert!(delivery.send(next(4)).is_stop());
    }

    #[test]
    fn test_an_observer_that_stops_drops_the_values_that_are_still_queued() {
        let log = new_log();
        let delivery = builder(&log).stop_after(2).build();

        assert!(
            delivery
                .send(EventBatch::NextBatch(vec![
                    Value::new(1),
                    Value::new(2),
                    Value::new(3).on_drop(note(&log, "3 dropped")),
                ]))
                .is_stop()
        );

        // Answering `Flow::Stop` stops the delivery, exactly as stopping it by hand does: the
        // observer, which the loop holds here, is released after what the state held.
        assert_eq!(values(&log), [1, 2]);
        assert_eq!(
            records(&log)[2..],
            [
                Record::Note("3 dropped"),
                Record::ResourcesDropped,
                Record::ObserverDropped,
            ]
        );
        assert!(delivery.send(next(4)).is_stop());
    }

    #[test]
    fn test_an_observer_that_stops_is_dropped_instead_of_being_terminated() {
        let log = new_log();
        let delivery = builder(&log).stop_after(1).build();

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::ObserverDropped,
            ],
            "an observer that ended its own stream must not be terminated on top of that"
        );
    }

    #[test]
    fn test_stop_from_on_next_suppresses_the_queued_termination() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    delivery.stop();
                }
            })
            .build();

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::ObserverDropped,
            ],
            "the observer must be dropped instead of being terminated"
        );
    }

    #[test]
    fn test_a_delivery_stop_leaves_the_observer_to_the_next_event() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();

        // A `DeliveryStop` cannot name the observer, so it only drops what the state held.
        delivery.stop_handle().stop();
        assert_eq!(records(&log), [Record::ResourcesDropped]);

        let rejected = Value::new(1).on_drop(note(&log, "1 dropped"));
        assert!(delivery.send(EventBatch::Next(rejected)).is_stop());
        // The drop of the observer re-entered the delivery, so it ran with the lock released.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::Note("1 dropped"),
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );

        assert!(delivery.send(next(2)).is_stop());
        assert_eq!(records(&log).len(), 4, "the observer is released only once");
    }

    #[test]
    fn test_a_delivery_stop_leaves_the_observer_to_the_next_update() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();

        delivery.stop_handle().stop();
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Stopped
        );
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );
    }

    #[test]
    fn test_a_delivery_stop_leaves_the_observer_to_a_later_stop() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();

        delivery.stop_handle().stop();
        delivery.stop();
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );

        delivery.stop();
        assert_eq!(records(&log).len(), 3, "the observer is released only once");
    }

    #[test]
    fn test_a_delivery_stop_leaves_the_observer_to_the_last_handle() {
        let log = new_log();
        let delivery = builder(&log).build();
        let stop = delivery.stop_handle();

        stop.stop();
        assert_eq!(records(&log), [Record::ResourcesDropped]);

        // The stop handle does not own the observer, so the last delivery handle releases it.
        drop(delivery);
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );
    }

    #[test]
    fn test_a_delivery_stop_leaves_the_observer_to_the_first_handle_dropped() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();
        let clone = delivery.clone();

        delivery.stop_handle().stop();
        assert_eq!(records(&log), [Record::ResourcesDropped]);

        // As a source drops its handle when it is disposed, while a scheduler task still holds
        // another: the first handle dropped after the stop releases the observer.
        drop(clone);
        // The drop of the observer re-entered the delivery, and dropped the handle it reached it
        // with, so it ran with the lock released.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );

        drop(delivery);
        assert_eq!(records(&log).len(), 3, "the observer is released only once");
    }

    #[test]
    fn test_a_handle_dropped_under_its_own_lock_takes_no_lock() {
        let log = new_log();
        let delivery = builder(&log).build();
        let clone = delivery.clone();

        // The update runs under the delivery's lock. Debug builds panic when a thread takes a lock
        // it already holds, so this passes only if dropping a handle of a delivery that has not
        // stopped takes no lock.
        assert_eq!(
            delivery.update(move |_| {
                drop(clone);
                UpdateOutcome::empty()
            }),
            Updated::Ran {
                result: (),
                flow: Flow::Continue
            }
        );
        assert_eq!(records(&log), []);
    }

    #[test]
    fn test_a_delivery_stop_from_on_next_lets_the_loop_drop_the_observer() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    delivery.stop_handle().stop();
                }
            })
            .build();

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_stop());
        // The loop holds the observer, and drops it once it sees the stopped state.
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::ObserverDropped,
            ],
            "the observer must be dropped instead of being terminated"
        );

        assert!(delivery.send(next(3)).is_stop());
        assert_eq!(records(&log).len(), 3, "the observer is released only once");
    }

    // MARK: - Re-entrant sends

    #[test]
    fn test_a_value_sent_from_on_next_is_delivered_after_the_queued_ones() {
        let log = new_log();
        let log_of_observer = log.clone();
        let delivery = builder(&log)
            .on_next(move |delivery, number| {
                if number == 1 {
                    assert!(delivery.send(next(10)).is_continue());
                    // The running loop picks it up: it is not delivered from inside this call.
                    assert_eq!(values(&log_of_observer), [1]);
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_continue());
        assert_eq!(values(&log), [1, 2, 3, 10]);
    }

    #[test]
    fn test_a_termination_sent_from_on_next_is_delivered_after_the_queued_values() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    // Queued behind this very delivery, the termination is delivered for certain,
                    // so the answer is already `Stop` — and nothing can be queued after it, even
                    // before it is delivered.
                    assert!(delivery.send(completed()).is_stop());
                    assert!(delivery.send(next(10)).is_stop());
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Next(2),
                Record::Next(3),
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    // MARK: - `update`

    #[test]
    fn test_update_without_events_updates_the_model_and_returns_its_result() {
        let log = new_log();
        let delivery = builder(&log).model(1).build();

        assert_eq!(
            delivery.update(|resources| {
                resources.model += 10;
                UpdateOutcome::new(resources.model)
            }),
            Updated::Ran {
                result: 11,
                flow: Flow::Continue
            }
        );
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Ran {
                result: 11,
                flow: Flow::Continue
            }
        );
        assert_eq!(records(&log), [], "an update alone delivers nothing");
    }

    #[test]
    fn test_update_runs_while_a_delivery_is_running() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                assert_eq!(
                    delivery.update(|resources| {
                        resources.model += number;
                        UpdateOutcome::new(resources.model)
                    }),
                    Updated::Ran {
                        result: number * (number + 1) / 2,
                        flow: Flow::Continue
                    }
                );
            })
            .build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_continue());
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Ran {
                result: 6,
                flow: Flow::Continue
            }
        );
    }

    #[test]
    fn test_update_without_events_behind_a_queued_termination_answers_stop() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    assert!(delivery.send(completed()).is_stop());
                    // The update still runs, since the resources live until the termination is
                    // delivered, but the stream it updates is over, which its flow says.
                    assert_eq!(
                        delivery.update(|resources| {
                            resources.model += 10;
                            UpdateOutcome::new(resources.model)
                        }),
                        Updated::Ran {
                            result: 10,
                            flow: Flow::Stop
                        }
                    );
                }
            })
            .build();

        assert!(delivery.send(next(1)).is_stop());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_update_after_a_termination_answers_stopped() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(completed()).is_stop());
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Stopped
        );
    }

    #[test]
    fn test_a_stopped_update_does_not_run_and_is_dropped_outside_the_lock() {
        let log = new_log();
        let delivery = builder(&log).build();
        delivery.stop();

        let probe = Value::new(0).on_drop(note_and_reenter(
            &DeliveryHandle::of(&delivery),
            &log,
            "update dropped",
        ));
        let result: Updated<()> = delivery.update(move |_resources| -> TestOutcome<()> {
            drop(probe); // Never runs: it is what makes the update own the probe.
            panic!("the update must not run once the delivery has stopped");
        });

        assert_eq!(result, Updated::Stopped);
        assert_eq!(
            records(&log)[2..],
            [Record::Note("update dropped")],
            "the update must be dropped, outside the lock"
        );
    }

    #[test]
    fn test_update_delivers_the_events_of_its_outcome() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert_eq!(
            delivery.update(|resources| {
                resources.model = 1;
                UpdateOutcome::new("first").with_next_event(Value::new(1))
            }),
            Updated::Ran {
                result: "first",
                flow: Flow::Continue
            }
        );
        assert_eq!(
            delivery.update(|_resources| {
                UpdateOutcome::empty()
                    .with_events(EventBatch::NextBatch(vec![Value::new(2), Value::new(3)]))
            }),
            Updated::Ran {
                result: (),
                flow: Flow::Continue
            }
        );
        assert_eq!(values(&log), [1, 2, 3]);
    }

    #[test]
    fn test_update_terminates_before_dropping_what_it_asked_to_drop_outside() {
        let log = new_log();
        let delivery = builder(&log).build();

        let result = delivery.update(|resources| {
            resources.model = 1;
            UpdateOutcome::empty()
                .with_drop_outside(Value::new(0).on_drop(note(&log, "dropped outside")))
                .with_next_and_termination_events(Value::new(1), Termination::Error(ERROR))
        });

        assert_eq!(
            result,
            Updated::Ran {
                result: (),
                flow: Flow::Stop
            }
        );
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Termination(Termination::Error(ERROR)),
                Record::ObserverDropped,
                Record::ResourcesDropped,
                // Dropped last: after the events it was queued with were delivered.
                Record::Note("dropped outside"),
            ]
        );
    }

    #[test]
    fn test_what_an_update_drops_outside_is_dropped_with_the_lock_released() {
        let log = new_log();
        let delivery = builder(&log).build();
        let probe = Value::new(0).on_drop(note_and_reenter(
            &DeliveryHandle::of(&delivery),
            &log,
            "dropped outside",
        ));

        let result = delivery.update(move |_resources| {
            UpdateOutcome::empty()
                .with_drop_outside(probe)
                .with_next_event(Value::new(1))
        });

        assert_eq!(
            result,
            Updated::Ran {
                result: (),
                flow: Flow::Continue
            }
        );
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Note("dropped outside"),
                // The delivery is parked again by then, so what that drop sent is delivered.
                Record::Next(99),
            ]
        );
    }

    #[test]
    fn test_update_with_a_termination_event_stops_the_delivery() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert_eq!(
            delivery.update(|_resources| {
                UpdateOutcome::empty().with_termination_event(Termination::Completed)
            }),
            Updated::Ran {
                result: (),
                flow: Flow::Stop
            }
        );
        assert_eq!(
            delivery.update(|_resources| UpdateOutcome::empty()),
            Updated::Stopped
        );
        assert!(delivery.send(next(1)).is_stop());
    }

    #[test]
    fn test_update_from_on_next_queues_its_events_after_the_pending_ones() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|delivery, number| {
                if number == 1 {
                    assert_eq!(
                        delivery.update(|resources| {
                            resources.model += 1;
                            UpdateOutcome::new(resources.model).with_next_event(Value::new(10))
                        }),
                        Updated::Ran {
                            result: 1,
                            flow: Flow::Continue
                        }
                    );
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2])).is_continue());
        assert_eq!(values(&log), [1, 2, 10]);
    }

    #[test]
    fn test_update_runs_and_returns_even_when_its_events_are_rejected() {
        let log = new_log();
        let log_of_observer = log.clone();
        let delivery = builder(&log)
            .on_next(move |delivery, number| {
                if number != 1 {
                    return;
                }
                assert!(delivery.send(completed()).is_stop());
                // The update still runs against the resources, but the termination is already
                // queued, so its events are dropped instead of being delivered, and its flow says
                // so.
                let dropped = Value::new(10).on_drop(note(&log_of_observer, "10 dropped"));
                assert_eq!(
                    delivery.update(move |resources| {
                        resources.model += 1;
                        UpdateOutcome::new(resources.model).with_next_event(dropped)
                    }),
                    Updated::Ran {
                        result: 1,
                        flow: Flow::Stop
                    }
                );
            })
            .build();

        assert!(delivery.send(next_batch([1, 2])).is_stop());
        assert_eq!(values(&log), [1, 2]);
        assert!(records(&log).contains(&Record::Note("10 dropped")));
    }

    #[test]
    fn test_an_update_answering_a_stop_delivers_its_events_and_leaves_the_delivery_open() {
        let log = new_log();
        let delivery = builder(&log).build();

        // The host stops this caller only: its value is still delivered, and the delivery goes on.
        let updated = delivery.update(|resources| {
            resources.model += 1;
            UpdateOutcome::new(resources.model)
                .with_next_event(Value::new(1))
                .answering_stop()
        });
        assert_eq!(
            updated,
            Updated::Ran {
                result: 1,
                flow: Flow::Stop
            }
        );
        assert_eq!(updated.result(), Some(1));
        assert!(delivery.send(next(2)).is_continue());
        assert_eq!(values(&log), [1, 2]);
    }

    #[test]
    fn test_the_arms_of_one_update_share_the_type_of_their_outcome() {
        let log = new_log();
        let delivery = builder(&log).build();

        // Both arms are one expression, so the type state makes the arm that emits nothing say so.
        let update = || {
            delivery.update(|resources| {
                resources.model += 1;
                if resources.model == 1 {
                    UpdateOutcome::new(resources.model)
                        .with_drop_outside(Value::new(0).on_drop(note(&log, "dropped outside")))
                        .with_next_event(Value::new(1))
                } else {
                    UpdateOutcome::new(resources.model)
                        .without_drop_outside()
                        .without_events()
                }
            })
        };

        assert_eq!(
            update(),
            Updated::Ran {
                result: 1,
                flow: Flow::Continue
            }
        );
        assert_eq!(
            update(),
            Updated::Ran {
                result: 2,
                flow: Flow::Continue
            }
        );
        assert_eq!(
            records(&log),
            [Record::Next(1), Record::Note("dropped outside")]
        );
    }

    // MARK: - Handles

    #[test]
    fn test_a_clone_shares_one_delivery() {
        let log = new_log();
        let delivery = builder(&log).build();
        let clone = delivery.clone();

        assert!(clone.send(next(1)).is_continue());
        delivery.stop();
        assert!(clone.send(next(2)).is_stop());
        assert_eq!(values(&log), [1]);
    }

    #[test]
    fn test_dropping_the_last_handle_drops_the_observer_without_notifying_it() {
        let log = new_log();
        let delivery = builder(&log).build();
        let clone = delivery.clone();

        drop(delivery);
        assert_eq!(records(&log), [], "another handle is still alive");

        drop(clone);
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );
    }

    // MARK: - Panicking observers

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_from_on_next_stops_the_delivery() {
        let log = new_log();
        let delivery = builder(&log)
            .on_next(|_delivery, number| {
                if number == 1 {
                    // Unwinds out of this notification without running the panic hook, which would
                    // print.
                    resume_unwind(Box::new(()));
                }
            })
            .build();

        let result = catch_unwind(AssertUnwindSafe(|| delivery.send(next_batch([1, 2, 3]))));
        assert!(result.is_err());

        // The delivery is stopped instead of being stuck in its delivering state: the values that
        // were still queued are gone, and so are the observer and the resources.
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::ObserverDropped,
            ]
        );
        assert!(delivery.send(next(4)).is_stop());
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_from_on_termination_drops_the_resources() {
        let log = new_log();
        let delivery = builder(&log)
            .on_termination(|_delivery, _termination| resume_unwind(Box::new(())))
            .build();

        let result = catch_unwind(AssertUnwindSafe(|| delivery.send(completed())));
        assert!(result.is_err());

        // Unwinding from the terminal notification drops the resources the loop was still holding.
        assert_eq!(
            records(&log),
            [
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
        assert!(delivery.send(next(1)).is_stop());
    }

    // MARK: - Concurrency

    #[test]
    fn test_concurrent_sends_are_delivered_one_at_a_time() {
        const THREADS: i32 = 4;
        const VALUES_PER_THREAD: i32 = 25;

        let log = new_log();
        let delivering = Arc::new(AtomicBool::new(false));
        let delivering_of_observer = delivering.clone();
        let delivery = builder(&log)
            .on_next(move |_delivery, _value| {
                assert!(
                    delivering_of_observer.set_if_changed(true),
                    "two values must never be delivered at the same time"
                );
                std::thread::yield_now();
                assert!(delivering_of_observer.set_if_changed(false));
            })
            .build();

        std::thread::scope(|scope| {
            for thread in 0..THREADS {
                let delivery = delivery.clone();
                scope.spawn(move || {
                    for value in 0..VALUES_PER_THREAD {
                        assert!(
                            delivery
                                .send(next(thread * VALUES_PER_THREAD + value))
                                .is_continue()
                        );
                    }
                });
            }
        });

        // Whichever thread sent it, and whichever thread delivered it, every value arrives exactly
        // once.
        let mut delivered = values(&log);
        delivered.sort_unstable();
        assert_eq!(
            delivered,
            (0..THREADS * VALUES_PER_THREAD).collect::<Vec<_>>()
        );
    }
}
