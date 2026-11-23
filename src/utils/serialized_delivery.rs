//! The state machine that serializes the events delivered to one observer, together with the lock
//! that guards it and the only operations allowed to drive it.
//!
//! [`SerializedDelivery`] is a shared handle. Its locks never leave this module, so a host cannot
//! hold one across a notification, a drop, or a second lock: every transition, the delivery loop,
//! and the rules of what may run under which lock live here.
//!
//! The observer lives apart from the rest of the state, which is what a [`DeliveryStop`] holds:
//! stopping a delivery from a disposal must not require naming the observer's type, since the
//! type of a disposal cannot depend on its observer (see
//! [`ObservableTypes`](crate::observable::ObservableTypes)). The rest of the state — the queue and
//! the host's resources — is the core, guarded by the delivery's lock. The observer stays in a cell
//! of its own, under a lock of its own, from the first event to the last, and the core says who
//! may lock that cell, so the two behave as one state:
//!
//! - An idle core leaves the cell to the delivery that the next event starts.
//! - A delivering core, or one with a termination queued behind its delivery, leaves it to the
//!   delivery loop, which locks the cell for as long as it runs and notifies the observer in place,
//!   taking the core's lock inside the cell's only to take the next batch or to find none left.
//!   Nobody else locks the cell meanwhile, so whatever a notification does to the delivery — an
//!   event, an update, a stop, a handle dropped — never meets that lock.
//! - A stopped core owns no resources, and says whether the observer is still parked in its cell
//!   with no loop to release it. Whoever finds it parked, under the core's lock, claims it, and
//!   releases it once that lock is released.
//!
//! So the cell's lock is never taken under the core's. The cell's is held across `on_next`, and
//! so across whatever `on_next` reaches — a stop it makes, and the resources that stop drops,
//! included. Nothing else the delivery calls runs under either lock — not `on_termination`, nor
//! any drop it makes — except the callback of an [`update`](SerializedDelivery::update), which runs
//! under the core's. A loop that parks its observer back turns the core idle under both locks, then
//! releases the cell with nothing run in between: whoever finds the core idle or stopped next waits
//! on the cell at most for that.
//!
//! Only an idle or a delivering core accepts events and updates. A raised flag implies a stopped
//! core; the reverse does not hold, since a termination stops the core without raising the flag.
//! [`StopView`] reports only the stops that raise it.
//!
//! # Drop order
//!
//! A delivery that stops drops, always outside the core's lock, what the stop finds in the core —
//! the queued events, then the resources — then the observer, unterminated, then the values the
//! observer will never see: the batch of the loop that ran, or the event or update that a call
//! finding the delivery over was rejected with.
//!
//! - A stop of the loop's own, its observer answering [`Flow::Break`] or unwinding, drops all of
//!   them in that order, on one thread.
//! - A [`DeliveryStop`] drops what it finds in the core, and whoever has the observer drops the
//!   rest: the loop running, or the call or handle that claims an observer left parked. Either may
//!   do so while the stop is still dropping the resources — from inside that drop, as a source
//!   drops its handle, or on another thread.
//!
//! The observer goes before the values since nobody else is left to take it out of its cell, which
//! a value whose drop unwinds must not prevent. A termination drops nothing before the observer is
//! notified of it: the queue is empty by then, and the resources are dropped once that is done. See
//! `docs/decisions/0011-serialized-delivery-in-batches.md`.
//!
//! # Updates
//!
//! A host that must change its own data, and emit the resulting events atomically, does so through
//! [`SerializedDelivery::update`], which runs its callback under the same lock that then queues
//! the events. The callback describes its outcome with an [`UpdateOutcome`], the one way to hand
//! this module events to queue and a value to drop after sending or queueing them, and the update
//! answers with an [`Updated`]: the callback's result, and whether the observer still accepts
//! events. Both live in a module of their own, which keeps the fields of an outcome out of reach
//! of the delivery. Updates run only while the delivery accepts events. Queueing a termination
//! freezes the host's data immediately, even while earlier values are being delivered.
//!
//! [`UpdateOutcome`] and [`Updated`] are what a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) takes and
//! answers too, so they are public, and re-exported by
//! [`subscribe_with_context`](crate::utils::subscribe_with_context); the delivery itself is not.

mod update_outcome;

use crate::{
    observer::{EventBatch, Flow, Observer, Termination},
    thread_mode::{
        ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
    utils::{
        flagged_lock::{FlagView, FlaggedLock},
        on_panic::on_panic,
    },
};
use educe::Educe;
use std::{collections::VecDeque, ops::ControlFlow};

pub use update_outcome::{DropDecided, DropUndecided, UpdateOutcome, Updated};

/// A shared, serialized delivery of events to one observer.
///
/// `R` is whatever the host owns alongside the observer, its sources' subscriptions typically. It
/// is dropped, outside the locks, once the delivery stops — after the terminal notification when the
/// delivery stops by terminating. See the [module documentation](self#drop-order) for the order in
/// which a stop drops everything.
///
/// A [`DeliveryStop`] cannot reach the observer: it leaves a parked one for the next event, update
/// or handle dropped — typically the handle of a source, as `R` disposes it — and one a delivery
/// loop is notifying to that loop.
///
/// Dropping the last delivery handle also releases the observer without a notification. A stop
/// handle may still own the core then, keeping `R` alive until it stops the core or is dropped:
/// letting go of the observer is not itself a disposal.
///
/// The pointers are the ones the thread mode `M` picks.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub(crate) struct SerializedDelivery<M: ThreadMode, T, E, OR, R> {
    /// The flag is raised by every stop but a termination, through [`stop_core`], and only there.
    /// Until a stop has run, nothing can be parked in a stopped core, so the `Drop` of a handle
    /// has nothing to release and takes no lock, and a delivery loop delivers the next value of
    /// its batch without the core's lock either.
    #[educe(Debug(ignore))]
    core: FlaggedLock<M, Core<T, E, R>>,
    /// The observer's cell, which only whoever the core designates locks: see the module
    /// documentation. It is empty once the observer has been terminated or released.
    // Declared after the core so that the last owner of both releases the resources first. A
    // DeliveryStop may keep only the core alive, letting the observer be released before them.
    #[educe(Debug(ignore))]
    observer_cell: M::Ptr<Option<OR>>,
}

/// Stops a [`SerializedDelivery`] without naming its observer's type: what a disposal holds.
///
/// Stopping drops the queued events and the resources, outside the lock, and rejects every later
/// event. The values a running delivery loop has already taken from the queue, to deliver them
/// without the core's lock, are out of its reach, as the observer is: the loop drops both once it
/// sees the stop, before selecting another value. A parked observer is released by the first
/// delivery handle dropped afterwards — typically at once, since dropping the resources disposes
/// the sources that hold those handles, even while a scheduler task's handle lives on — or by the
/// next event or update that arrives, whichever comes first.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub(crate) struct DeliveryStop<M: ThreadMode, T, E, R> {
    #[educe(Debug(ignore))]
    core: FlaggedLock<M, Core<T, E, R>>,
}

/// Sends to an observer only while a delivery has not been stopped other than by its termination,
/// a stop it reads without a lock: one by a [`DeliveryStop`], or by the delivery's own loop, as its
/// observer answers [`Flow::Break`] or panics.
///
/// While the observer runs, only the first can happen, so the observer learns through this view
/// that its subscription has been disposed — from inside one of its own notifications, or from
/// another thread meanwhile — which the delivery itself checks between two events only. An
/// observer that emits several events for one, or runs user code before an emission, sends each
/// of them through [`send`](Self::send), [`send_each`](Self::send_each) or
/// [`terminate`](Self::terminate), which check the stop before and after that code: they are the
/// only way to read it outside this module. A disposal during the termination counts too: a stop
/// raises its flag whatever state it finds the core in, a termination that has already stopped it
/// included.
///
/// The flag is raised under the core's lock, before the stop drops what the core held, so code that runs
/// as the resources are dropped — the disposal of a source, and a user callback it runs — sees the
/// delivery stopped.
pub(crate) struct StopView<M: ThreadMode>(FlagView<M>);

impl<M: ThreadMode> StopView<M> {
    /// Whether the delivery has been stopped, for good, other than by its termination.
    fn is_stopped(&self) -> bool {
        self.0.is_raised()
    }

    /// Sends `observer` the value that `value` makes, unless the delivery is stopped before
    /// `value` runs or while it runs — user code, a selector or a `Clone`, which may dispose the
    /// subscription. [`Flow::Break`] when it is stopped, and the value is then not made, or dropped
    /// unsent, or when `observer` answered it.
    pub(crate) fn send<T, E>(
        &self,
        observer: &mut impl Observer<T, E>,
        value: impl FnOnce() -> T,
    ) -> Flow {
        if self.is_stopped() {
            return Flow::Break(());
        }
        let value = value();
        if self.is_stopped() {
            return Flow::Break(());
        }
        observer.on_next(value)
    }

    /// Sends the value that `value` makes for each of `items`, until the delivery is stopped or
    /// `observer` answers [`Flow::Break`]. Each value is made inside [`send`](Self::send), so a stop
    /// before it is needed skips the user's code too.
    pub(crate) fn send_each<T, E, V>(
        &self,
        observer: &mut impl Observer<T, E>,
        items: impl IntoIterator<Item = V>,
        mut value: impl FnMut(V) -> T,
    ) -> Flow {
        items
            .into_iter()
            .try_for_each(|item| self.send(observer, || value(item)))
    }

    /// [`send`](Self::send) for the termination that `termination` makes: `observer` is dropped
    /// unterminated when the delivery is stopped before or while `termination` runs.
    pub(crate) fn terminate<T, E>(
        &self,
        observer: impl Observer<T, E>,
        termination: impl FnOnce() -> Termination<E>,
    ) {
        if self.is_stopped() {
            return;
        }
        let termination = termination();
        if self.is_stopped() {
            return;
        }
        observer.on_termination(termination);
    }
}

/// Stops `core`, raising its flag under the same lock, then drops what the core held — the queued
/// events, then the resources — once the lock is released. Stopping again is a no-op.
///
/// Every stop goes through here: a [`DeliveryStop`], and the delivery loop as its observer stops
/// or unwinds. The observer is not dropped here: the first cannot name its type, and leaves an idle
/// delivery's parked; the second holds it, and drops it once this returns. Only a delivery that
/// terminates ends elsewhere, in its loop, which consumes the observer. So the flag of the core is
/// raised exactly when it has stopped other than by a termination.
fn stop_core<M: ThreadMode, T, E, R>(core: &FlaggedLock<M, Core<T, E, R>>) {
    let previous = core.raise_with(Core::stop);
    // Outside the lock to avoid potential deadlock, and after the stop, so that what a drop sends
    // is rejected.
    drop(previous);
}

/// Everything a delivery holds except its observer, and who may lock the observer's cell.
///
/// `Idle` and `Delivering` accept events and updates; `TerminationQueued` and `Stopped` reject both.
enum Core<T, E, R> {
    /// No delivery is running. The observer waits in its cell for the delivery the next event
    /// starts.
    Idle { resources: R },
    /// A delivery loop has the observer's cell to itself. The values sent meanwhile — from inside a
    /// notification, or from another thread — wait in `queued`.
    Delivering { queued: VecDeque<T>, resources: R },
    /// As `Delivering`, with `termination` queued after `queued`: the stream's end is decided, so
    /// every later event and update is rejected. The loop delivers what is queued, then the
    /// termination, and only then drops `resources`.
    TerminationQueued {
        queued: VecDeque<T>,
        termination: Termination<E>,
        resources: R,
    },
    /// The resources have been moved out to be dropped or used outside the lock. Every later
    /// event is rejected; their drop need not have finished yet.
    Stopped {
        /// Whether the observer is still in its cell with no delivery loop to release it, as a
        /// [`DeliveryStop`] leaves an idle delivery's. Whoever finds it set under the lock — an
        /// event, an update, or a handle being dropped — clears it with
        /// [`claim_parked_observer`](Core::claim_parked_observer), which makes it the only one
        /// left to lock the cell, and releases the observer once the lock is released.
        observer_parked: bool,
    },
}

/// The action to perform after releasing the lock used to call `enqueue_batch`.
enum EnqueueAction<T, E> {
    /// Start a delivery loop, which delivers `first_next` before anything queued.
    Start { first_next: Option<T> },
    /// Nothing to carry out: the batch was queued for a running delivery, or needed no work — an
    /// empty batch, or an update that decided no events. Answer this flow: [`Flow::Break`] exactly
    /// when the termination was queued.
    Return(Flow),
    /// The delivery was already stopped or a termination was already queued: release the observer,
    /// when this call claimed one a [`DeliveryStop`] left parked, then drop `events`.
    Rejected {
        events: EventBatch<T, E>,
        release_parked_observer: bool,
    },
}

/// How a delivery loop ended, decided under the lock of the observer's cell and carried out once
/// it is released, so that the observer is terminated or dropped with no lock held.
enum LoopEnd<E, R> {
    /// Nothing is queued: the core is idle, and the observer stays in its cell for the next
    /// delivery.
    Parked,
    /// Stop the delivery, unless it has stopped already, and drop the observer without terminating
    /// it: the loop found a stop, or the observer answered [`Flow::Break`], having ended its own
    /// stream or been disposed.
    Stop,
    /// Terminate the observer, then drop `resources`. The core is already `Stopped`.
    Terminate {
        termination: Termination<E>,
        resources: R,
    },
}

impl<M: ThreadMode, T, E, R> DeliveryStop<M, T, E, R> {
    /// Stops the delivery. Stopping again is a no-op.
    pub(crate) fn stop(&self) {
        // The observer's type is not known here, so an observer parked in its cell stays there
        // for whoever finds the stop next.
        stop_core(&self.core);
    }
}

impl<M: ThreadMode, T, E, OR, R> SerializedDelivery<M, T, E, OR, R> {
    /// Starts idle, waiting for the first event, with the observer that `observer_builder` builds
    /// around the [`StopView`] of the delivery parked in its cell. An observer with no use for the
    /// view ignores it.
    pub(crate) fn idle(observer_builder: impl FnOnce(StopView<M>) -> OR, resources: R) -> Self {
        let core = FlaggedLock::new(Core::Idle { resources });
        Self {
            observer_cell: M::ptr(Some(observer_builder(StopView(core.view())))),
            core,
        }
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
    pub(crate) fn accepts_events(&self) -> bool {
        self.core.with_mut(|core| core.accepts_events())
    }

    /// Takes the observer out of its cell, if it is still there, and drops it once the cell is
    /// unlocked: for whoever claimed a parked observer, under the core's lock it has released. The
    /// claim leaves nobody else to take the observer out, so this comes before anything else the
    /// claimer drops, which may unwind.
    ///
    /// A loop that has just parked the observer back may still be releasing the cell, so taking
    /// the observer out can wait on it for that long, which takes no other lock.
    fn release_observer(&self) {
        let observer = self.observer_cell.take_value();
        drop(observer); // Drop outside the lock to avoid potential deadlock
    }
}

impl<M: ThreadMode, T, E, OR, R> SerializedDelivery<M, T, E, OR, R>
where
    OR: Observer<T, E>,
{
    /// Queues `events` and delivers whatever became deliverable because of them.
    ///
    /// Returns whether the observer still accepts events. [`Flow::Break`] means one of three
    /// things: the delivery had stopped or a termination was already queued, so the events were
    /// rejected and dropped outside the lock; `events` itself carried a termination, since nothing
    /// can be queued after it; or the delivery this call ran answered it, its observer having
    /// stopped. A value queued behind a delivery running elsewhere is reported as
    /// [`Flow::Continue`]; see [`Flow`].
    pub(crate) fn send(&self, events: EventBatch<T, E>) -> Flow {
        let action = self.core.with_mut(|core| core.enqueue_batch(events));
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
    /// Returns the callback result and flow described by [`Updated::Ran`]. If the delivery has
    /// stopped or a termination is already queued, returns [`Updated::Ended`] and drops the
    /// unused callback and its captures outside the lock.
    ///
    /// Captures of a skipped update can drop before a queued termination reaches the observer.
    /// The delivery still owns its existing resources until it terminates or is stopped.
    pub(crate) fn update<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(&mut R) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Updated<Out> {
        let outcome = self.core.with_mut(|core| {
            let Some(resources) = core.active_resources_mut() else {
                // `update` goes back out, so that its captures are dropped only after the lock is
                // released, together with the claim on a parked observer, if any.
                return Err((update, core.claim_parked_observer()));
            };
            let (events, drop_outside, answered, result) = update(resources).into_parts();
            let action = match events {
                Some(events) => core.enqueue_batch(events),
                None => EnqueueAction::Return(Flow::Continue(())),
            };
            Ok((action, drop_outside, answered, result))
        });
        let (action, drop_outside, answered, result) = match outcome {
            Ok(outcome) => outcome,
            Err((update, release_parked_observer)) => {
                if release_parked_observer {
                    self.release_observer();
                }
                drop(update); // After the observer, even if its drop unwinds, and outside the lock.
                return Updated::Ended;
            }
        };
        let delivered = self.perform(action);
        drop(drop_outside); // Drop after sending or queueing the events, outside the lock.
        let flow = if answered.is_break() {
            Flow::Break(())
        } else {
            delivered
        };
        Updated::Ran { result, flow }
    }

    /// Carries out, with the lock released, what `enqueue_batch` decided under it.
    ///
    /// A batch that carries a termination answers [`Flow::Break`] on every path: a delivery it
    /// starts ends by terminating, one it is queued behind answers `Return(Flow::Break(()))`, and a
    /// rejection is a stop.
    fn perform(&self, action: EnqueueAction<T, E>) -> Flow {
        match action {
            EnqueueAction::Start { first_next } => self.deliver(first_next),
            EnqueueAction::Return(flow) => flow,
            EnqueueAction::Rejected {
                events,
                release_parked_observer,
            } => {
                if release_parked_observer {
                    self.release_observer();
                }
                drop(events); // After the observer, even if its drop unwinds, and outside the lock.
                Flow::Break(())
            }
        }
    }

    /// Delivers `next`, then every queued event, one at a time.
    ///
    /// The loop locks the observer's cell for as long as it runs, and notifies the observer in
    /// place: the core left the cell to it as it turned `Delivering`, so nobody else locks the cell
    /// until the loop ends. Each time it runs out of values, the loop takes the core's lock once
    /// and moves every value queued so far into a batch of its own, which it then delivers without
    /// that lock; the termination stays with the core, `TerminationQueued` from the moment it was queued,
    /// so nothing can be queued after it. Events arriving during the delivery queue up behind the
    /// batch, so they are delivered in arrival order.
    ///
    /// A stop takes effect between any two values all the same — even two of an
    /// `EventBatch::NextBatch`: it raises the flag of the core, which the loop reads, without the
    /// core's lock, between values of its batch, and finds `Stopped` when it locks for the next
    /// batch. The loop then drops its observer and its batch instead of feeding it further. A value
    /// selected before the stop may still be delivered: stopping does not wait for an in-flight
    /// notification. See the [module documentation](self#drop-order) for the order of the drops.
    ///
    /// `on_next` runs under the cell's lock alone, which nothing it reaches takes. The termination,
    /// and every drop that ends the loop, run once the cell is unlocked: every end but parking then
    /// takes the observer out of it, which nobody else does. An `on_next` that unwinds stops the
    /// delivery, so a caught panic does not leave it stuck in the delivering state; the guard runs
    /// once the unwind has released the cell, so it can take the observer out too.
    /// `on_termination` needs no guard: the state is already stopped when it runs, and the unwind
    /// drops the resources.
    ///
    /// Returns [`Flow::Break`] when this delivery ended — the observer stopped, it was terminated,
    /// or the delivery had stopped already — and [`Flow::Continue`] when the observer was parked
    /// back for the next event.
    fn deliver(&self, next: Option<T>) -> Flow {
        // The values taken from the queue under one lock, delivered one at a time without it. It
        // trades its allocation with the queue's at each batch, so `new` allocates nothing here.
        // Both buffers are released when the loop terminates or parks its observer.
        //
        // Declared before the guard, whose action drops the observer, so that an unwind drops the
        // batch after it, as the returning paths do. A local rather than the state of the guard,
        // which measured about a tenth slower per value on a `Local` delivery.
        let mut batch = VecDeque::new();
        let stop_on_panic = on_panic(|| self.stop_from_loop());
        let end = self.observer_cell.with_mut(|cell| {
            let observer = cell.as_mut().expect(OBSERVER_IN_CELL);
            self.deliver_in_cell(observer, next, &mut batch)
        });
        stop_on_panic.disarm();

        match end {
            LoopEnd::Parked => return Flow::Continue(()),
            LoopEnd::Stop => self.stop_from_loop(),
            LoopEnd::Terminate {
                termination,
                resources,
            } => {
                let observer = self.observer_cell.take_value().expect(OBSERVER_IN_CELL);
                observer.on_termination(termination);
                // The resources outlive the terminal notification, so a host can dispose its
                // source only after its downstream was told the stream ended.
                drop(resources);
            }
        }
        Flow::Break(())
    }

    /// Stops the delivery, unless it has stopped already, then drops the observer, unterminated,
    /// even if the stop unwinds as it drops the resources: for the delivery loop that has the cell
    /// to itself, once it has unlocked it.
    fn stop_from_loop(&self) {
        let observer = self.observer_cell.take_value();
        stop_core(&self.core);
        drop(observer);
    }

    /// The loop of [`deliver`](Self::deliver), run under the lock of the observer's cell, which the
    /// caller releases as soon as this returns: once the loop has parked the core, whoever finds it
    /// idle or stopped next may be waiting for the cell.
    fn deliver_in_cell(
        &self,
        observer: &mut OR,
        mut next: Option<T>,
        batch: &mut VecDeque<T>,
    ) -> LoopEnd<E, R> {
        loop {
            if let Some(value) = next.take()
                && observer.on_next(value).is_break()
            {
                return LoopEnd::Stop;
            }
            if batch.is_empty() {
                if let ControlFlow::Break(end) =
                    self.core.with_mut(|core| core.refill_or_end(batch))
                {
                    return end;
                }
            } else if self.core.is_raised() {
                // A stop raises the flag under the core's lock, as it turns the core `Stopped`, so
                // a lowered flag is all the next value needs, and it takes no lock.
                return LoopEnd::Stop;
            }
            next = batch.pop_front();
        }
    }
}

/// What a delivery loop expects of its observer's cell.
const OBSERVER_IN_CELL: &str = "a delivery loop's observer stays in its cell until the loop ends";

impl<M: ThreadMode, T, E, OR, R> Drop for SerializedDelivery<M, T, E, OR, R> {
    /// Releases an observer that a [`DeliveryStop`] left parked.
    ///
    /// A host whose handles are all held by its sources gets this anyway, from the last one. One
    /// that a scheduler task also holds gets it from the first handle dropped after the stop: the
    /// handle of the source, as the source is disposed, so the observer is released before
    /// `dispose` returns rather than when the runtime gets round to dropping the cancelled task.
    ///
    /// Only a [`DeliveryStop`] leaves the observer parked in a stopped delivery, so until a stop
    /// has run, which the flag of the [`FlaggedLock`] says without a lock, there is nothing to
    /// release and no lock is taken. A handle can therefore be dropped anywhere, even under the
    /// core's lock: after the stop, nothing runs under that lock that could drop one. And under the
    /// cell's — by the observer a delivery loop is notifying — since a stop that finds a loop
    /// running leaves no observer parked for the drop to release.
    fn drop(&mut self) {
        let release_parked_observer = self.core.if_raised(|core| {
            debug_assert!(matches!(core, Core::Stopped { .. }), "a stop is final");
            core.claim_parked_observer()
        });
        if release_parked_observer == Some(true) {
            self.release_observer();
        }
    }
}

impl<T, E, R> Core<T, E, R> {
    /// Whether events and updates are still accepted: no termination is queued, and the delivery
    /// has not stopped.
    fn accepts_events(&self) -> bool {
        matches!(self, Self::Idle { .. } | Self::Delivering { .. })
    }

    /// The resources, while the delivery [accepts events](Self::accepts_events).
    fn active_resources_mut(&mut self) -> Option<&mut R> {
        match self {
            Self::Idle { resources } | Self::Delivering { resources, .. } => Some(resources),
            Self::TerminationQueued { .. } | Self::Stopped { .. } => None,
        }
    }

    /// Moves the state out to rebuild it, leaving a placeholder that the caller replaces under
    /// the same lock, so nobody sees it.
    fn take(&mut self) -> Self {
        std::mem::replace(
            self,
            Self::Stopped {
                observer_parked: false,
            },
        )
    }

    /// Turns the core `Stopped` and returns what it held. An idle core's observer is left parked
    /// in its cell; a running loop keeps its own; a stopped core keeps what it says.
    fn stop(&mut self) -> Self {
        let observer_parked = match self {
            Self::Idle { .. } => true,
            Self::Delivering { .. } | Self::TerminationQueued { .. } => false,
            Self::Stopped { observer_parked } => *observer_parked,
        };
        std::mem::replace(self, Self::Stopped { observer_parked })
    }

    /// Whether the caller has just claimed an observer that a stop left parked, to release it
    /// once this lock is released: it is then the only one left to lock the cell.
    fn claim_parked_observer(&mut self) -> bool {
        match self {
            Self::Stopped { observer_parked } => std::mem::take(observer_parked),
            Self::Idle { .. } | Self::Delivering { .. } | Self::TerminationQueued { .. } => false,
        }
    }

    /// Queues `events`, or tells the caller to start a delivery loop with them when the core is
    /// idle. A batch that carries a termination moves the core to `TerminationQueued`.
    fn enqueue_batch(&mut self, events: EventBatch<T, E>) -> EnqueueAction<T, E> {
        match self {
            // The delivery loop picks these events up on its own.
            Self::Delivering { queued, .. } => match queue_values(events, queued) {
                None => EnqueueAction::Return(Flow::Continue(())),
                Some(termination) => {
                    let Self::Delivering { queued, resources } = self.take() else {
                        unreachable!()
                    };
                    *self = Self::TerminationQueued {
                        queued,
                        termination,
                        resources,
                    };
                    EnqueueAction::Return(Flow::Break(()))
                }
            },
            Self::TerminationQueued { .. } | Self::Stopped { .. } => EnqueueAction::Rejected {
                events,
                release_parked_observer: self.claim_parked_observer(),
            },
            Self::Idle { .. } => {
                let (first_next, queued, termination) = split_batch(events);
                if first_next.is_none() && termination.is_none() {
                    // An empty `NextBatch` is a no-op, consistent with the delivering state.
                    return EnqueueAction::Return(Flow::Continue(()));
                }

                let Self::Idle { resources } = self.take() else {
                    unreachable!()
                };
                *self = match termination {
                    None => Self::Delivering { queued, resources },
                    Some(termination) => Self::TerminationQueued {
                        queued,
                        termination,
                        resources,
                    },
                };
                EnqueueAction::Start { first_next }
            }
        }
    }

    /// Refills `batch`, which a running delivery loop has emptied, with the values queued so far, by
    /// swapping the two; once none are left, or the delivery has stopped, breaks with how the loop
    /// ends: parked as the core turns `Idle`, stopping, or with the termination and the resources as
    /// the core turns `Stopped`. `batch` stays
    /// with the loop either way, so its buffer is released outside the lock.
    fn refill_or_end(&mut self, batch: &mut VecDeque<T>) -> ControlFlow<LoopEnd<E, R>> {
        debug_assert!(
            batch.is_empty(),
            "a batch is refilled once the last one is done"
        );
        match self {
            Self::Delivering { queued, .. } | Self::TerminationQueued { queued, .. }
                if !queued.is_empty() =>
            {
                std::mem::swap(queued, batch);
                return ControlFlow::Continue(());
            }
            Self::Stopped { observer_parked } => {
                debug_assert!(!*observer_parked, "a running loop keeps its observer");
                return ControlFlow::Break(LoopEnd::Stop);
            }
            Self::Idle { .. } => unreachable!("a delivery loop only runs while delivering"),
            Self::Delivering { .. } | Self::TerminationQueued { .. } => {}
        }

        // Everything the state owns is handed to the caller or put back into `Idle` below; the
        // empty queue dropped here owns no user value.
        match self.take() {
            Self::Delivering { resources, .. } => {
                *self = Self::Idle { resources };
                ControlFlow::Break(LoopEnd::Parked)
            }
            // The loop terminates its observer: none is left parked.
            Self::TerminationQueued {
                termination,
                resources,
                ..
            } => ControlFlow::Break(LoopEnd::Terminate {
                termination,
                resources,
            }),
            Self::Idle { .. } | Self::Stopped { .. } => unreachable!(),
        }
    }
}

/// Splits `events` for a delivery about to start: the value it delivers first, the values queued
/// behind it, and the termination. The first value never enters the queue, so a batch of one
/// value allocates no queue at all, and a `Vec` becomes the queue without being copied.
fn split_batch<T, E>(events: EventBatch<T, E>) -> (Option<T>, VecDeque<T>, Option<Termination<E>>) {
    let (values, termination) = match events {
        EventBatch::Next(value) => return (Some(value), VecDeque::new(), None),
        EventBatch::NextAndTermination(value, termination) => {
            return (Some(value), VecDeque::new(), Some(termination));
        }
        EventBatch::Termination(termination) => return (None, VecDeque::new(), Some(termination)),
        EventBatch::NextBatch(values) => (values, None),
        EventBatch::NextBatchAndTermination(values, termination) => (values, Some(termination)),
    };
    let mut values = VecDeque::from(values);
    (values.pop_front(), values, termination)
}

/// Appends the values of `events` to `queued`, behind those of a running delivery, and returns
/// its termination.
fn queue_values<T, E>(
    events: EventBatch<T, E>,
    queued: &mut VecDeque<T>,
) -> Option<Termination<E>> {
    match events {
        EventBatch::Next(value) => {
            queued.push_back(value);
            None
        }
        EventBatch::Termination(termination) => Some(termination),
        EventBatch::NextAndTermination(value, termination) => {
            queued.push_back(value);
            Some(termination)
        }
        EventBatch::NextBatch(values) => {
            queued.extend(values);
            None
        }
        EventBatch::NextBatchAndTermination(values, termination) => {
            queued.extend(values);
            Some(termination)
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests of the delivery on its own, without any operator around it.
    //
    // They drive the state machine directly, so they can check what only this module promises: the
    // order in which queued and re-entrant events reach the observer, when the observer and the
    // resources are dropped, and that nothing is ever dropped while either lock is held, nor
    // notified while the core is locked.
    //
    // The last one is checked with values whose `Drop` re-enters the delivery: a drop that ran
    // under a lock would deadlock, or fail the re-entry check of debug builds, instead of
    // recording its note.

    use super::{SerializedDelivery, StopView, UpdateOutcome, Updated};
    use crate::{
        observer::{EventBatch, Flow, Observer, Termination, boxed_observer::SendBoxedObserver},
        thread_mode::{
            Shared,
            mutable::{MutableExt, MutableHelper},
        },
    };
    #[cfg(panic = "unwind")]
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        time::Duration,
    };

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

    /// The [`StopView`] of a delivery to be built, for a callback that has to be made before it,
    /// filled in as it is built.
    #[derive(Clone)]
    struct StopViewHandle(Arc<Mutex<Option<StopView<Shared>>>>);

    impl StopViewHandle {
        fn is_stopped(&self) -> bool {
            self.0.with_ref(|view| {
                view.as_ref()
                    .expect("the view is installed before anything can read it")
                    .is_stopped()
            })
        }

        /// Takes the view out, for a test that sends through it rather than reads it from a
        /// callback.
        fn take(&self) -> StopView<Shared> {
            self.0
                .take_value()
                .expect("the view is installed as the delivery is built")
        }
    }

    /// What a test sends to through a [`StopView`]: it records into its log, and runs `on_next`
    /// on each value, after recording it.
    struct Target<F> {
        log: Log,
        on_next: F,
    }

    fn target<F: FnMut(i32)>(log: &Log, on_next: F) -> Target<F> {
        Target {
            log: log.clone(),
            on_next,
        }
    }

    impl<F: FnMut(i32)> Observer<i32, TestError> for Target<F> {
        fn on_next(&mut self, value: i32) -> Flow {
            record(&self.log, Record::Next(value));
            (self.on_next)(value);
            Flow::Continue(())
        }

        fn on_termination(self, termination: Termination<TestError>) {
            record(&self.log, Record::Termination(termination));
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
        /// How many values this observer accepts before it answers [`Flow::Break`], if it ever does.
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
                Some(stop_after) if self.received >= stop_after => Flow::Break(()),
                _ => Flow::Continue(()),
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
            stop_view: StopViewHandle(Arc::new(Mutex::new(None))),
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
        stop_view: StopViewHandle,
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

        /// The stop view of the delivery to be built.
        fn stop_view(&self) -> StopViewHandle {
            self.stop_view.clone()
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
                stop_view: self.stop_view,
                reaches_delivery: true,
                model: self.model,
                on_next,
                on_termination: self.on_termination,
                on_observer_drop: self.on_observer_drop,
                stop_after: self.stop_after,
            }
        }

        /// Makes the observer answer [`Flow::Break`] once it has received `count` values.
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
                stop_view: self.stop_view,
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
            let delivery = SerializedDelivery::idle(
                |view| {
                    self.stop_view.0.replace_value(Some(view));
                    observer
                },
                resources,
            );
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

        assert!(delivery.send(completed()).is_break());
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
                .is_break()
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
                .is_break()
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

        assert!(delivery.send(completed()).is_break());
        assert!(delivery.send(next(1)).is_break());
        assert!(delivery.send(errored()).is_break());
        assert!(delivery.send(EventBatch::NextBatch(vec![])).is_break());
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

        delivery.stop_handle().stop();
        assert!(delivery.send(next(1)).is_break());
        assert!(delivery.send(completed()).is_break());
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );
    }

    #[test]
    fn test_rejected_events_are_dropped_outside_the_lock() {
        let log = new_log();
        let delivery = builder(&log).build();
        assert!(delivery.send(completed()).is_break());

        let rejected = Value::new(1).on_drop(note_and_reenter(
            &DeliveryHandle::of(&delivery),
            &log,
            "rejected dropped",
        ));
        assert!(delivery.send(EventBatch::Next(rejected)).is_break());

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
                assert!(delivery.send(next(1)).is_break());
                assert_eq!(
                    delivery.update(|_resources| UpdateOutcome::empty()),
                    Updated::Ended
                );
            })
            .build();

        assert!(delivery.send(completed()).is_break());
        assert_eq!(values(&log), []);
    }

    // MARK: - Stopping

    #[test]
    fn test_a_delivery_stop_is_idempotent() {
        let log = new_log();
        let delivery = builder(&log).build();
        let stop = delivery.stop_handle();

        stop.stop();
        assert_eq!(records(&log), [Record::ResourcesDropped]);

        stop.stop();
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped],
            "stopping again must drop nothing more"
        );

        // The observer is released by the next event, once, and never notified.
        assert!(delivery.send(next(1)).is_break());
        assert!(delivery.send(next(2)).is_break());
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::ObserverDropped]
        );
    }

    #[test]
    fn test_a_delivery_stop_drops_the_resources_outside_the_lock() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder.build();
        delivery
            .update(|resources| {
                resources.on_drop(note_and_reenter(&handle, &log, "resources dropped"));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");

        delivery.stop_handle().stop();

        // The drop re-entered the delivery, so it did not run while the state was locked. What it
        // sent is the next event, which releases the observer the stop left parked.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::Note("resources dropped"),
                Record::ObserverDropped,
            ]
        );
    }

    #[test]
    fn test_stop_from_on_next_drops_the_values_that_are_still_queued() {
        let log = new_log();
        let log_of_queued = log.clone();
        let delivery = builder(&log)
            .on_next(move |delivery, number| {
                if number == 2 {
                    // Queued behind the batch the loop holds, so the stop finds it in the queue.
                    let queued = Value::new(4).on_drop(note(&log_of_queued, "4 dropped"));
                    assert!(delivery.send(EventBatch::Next(queued)).is_continue());
                    delivery.stop_handle().stop();
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
                .is_break()
        );

        assert_eq!(values(&log), [1, 2]);
        assert_eq!(
            records(&log)[2..],
            [
                // The stop drops what the queue still holds, then the resources.
                Record::Note("4 dropped"),
                Record::ResourcesDropped,
                // The loop took 3 from the queue with 2, so the stop cannot reach it: the loop
                // drops the observer it holds, then 3, once it sees the stop, before delivering it.
                Record::ObserverDropped,
                Record::Note("3 dropped"),
            ]
        );
        assert!(delivery.send(next(4)).is_break());
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
                .is_break()
        );

        // Answering `Flow::Break(())` stops the delivery, exactly as a `DeliveryStop` does: the
        // observer the loop holds, then 3, which it took with 2, go after what the state held.
        assert_eq!(values(&log), [1, 2]);
        assert_eq!(
            records(&log)[2..],
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("3 dropped"),
            ]
        );
        assert!(delivery.send(next(4)).is_break());
    }

    #[test]
    fn test_an_observer_that_stops_drops_the_resources_outside_the_lock() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .stop_after(1)
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();
        delivery
            .update(|resources| {
                resources.on_drop(note_and_reenter(&handle, &log, "resources dropped"));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");

        assert!(delivery.send(next(1)).is_break());

        // Both drops re-entered the delivery, so neither of them ran while the state was locked.
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::Note("resources dropped"),
                Record::ObserverDropped,
                Record::Note("observer dropped"),
            ]
        );
    }

    #[test]
    fn test_an_observer_that_stops_is_dropped_instead_of_being_terminated() {
        let log = new_log();
        let delivery = builder(&log).stop_after(1).build();

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_break());
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
                    delivery.stop_handle().stop();
                }
            })
            .build();

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_break());
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
        assert!(delivery.send(EventBatch::Next(rejected)).is_break());
        // The drop of the observer re-entered the delivery, so it ran with the lock released.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
                Record::Note("1 dropped"),
            ]
        );

        assert!(delivery.send(next(2)).is_break());
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
            Updated::Ended
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
                flow: Flow::Continue(())
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

        assert!(delivery.send(next_batch_and_completed([1, 2])).is_break());
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

        assert!(delivery.send(next(3)).is_break());
        assert_eq!(records(&log).len(), 3, "the observer is released only once");
    }

    #[test]
    fn test_what_a_stopped_observer_sends_or_drops_leaves_its_cell_to_the_loop() {
        let log = new_log();
        let log_of_observer = log.clone();
        let delivery = builder(&log)
            .on_next(move |delivery, number| {
                if number == 1 {
                    delivery.stop_handle().stop();
                    // The loop notifies the observer under the lock of its cell, which none of
                    // these may take: debug builds would panic, and a `Mutex` would deadlock.
                    let rejected = Value::new(2).on_drop(note(&log_of_observer, "2 dropped"));
                    assert!(delivery.send(EventBatch::Next(rejected)).is_break());
                    assert_eq!(
                        delivery.update(|resources| UpdateOutcome::new(resources.model)),
                        Updated::Ended
                    );
                    drop(delivery.clone());
                }
            })
            .build();

        assert!(delivery.send(next(1)).is_break());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::Note("2 dropped"),
                // None of them took the observer: the loop releases it once the hook returns.
                Record::ObserverDropped,
            ]
        );
    }

    // MARK: - The stop view

    #[test]
    fn test_a_delivery_stop_raises_the_stop_view_before_the_resources_are_dropped() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let delivery = builder.build();
        let stop_view_of_resources = stop_view.clone();
        let log_of_resources = log.clone();
        delivery
            .update(move |resources| {
                resources.on_drop(Box::new(move || {
                    if stop_view_of_resources.is_stopped() {
                        record(&log_of_resources, Record::Note("raised"));
                    }
                }));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");
        assert!(!stop_view.is_stopped());

        delivery.stop_handle().stop();

        assert!(stop_view.is_stopped());
        assert_eq!(
            records(&log),
            [Record::ResourcesDropped, Record::Note("raised")],
            "the view must be raised before the resources are dropped"
        );
    }

    #[test]
    fn test_a_delivery_stop_from_on_next_raises_the_stop_view_at_once() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let stop_view_of_observer = stop_view.clone();
        let log_of_observer = log.clone();
        let delivery = builder
            .on_next(move |delivery, number| {
                if number == 1 {
                    delivery.stop_handle().stop();
                    if stop_view_of_observer.is_stopped() {
                        record(&log_of_observer, Record::Note("raised"));
                    }
                }
            })
            .build();

        assert!(delivery.send(next(1)).is_break());
        assert_eq!(
            records(&log)[..3],
            [
                Record::Next(1),
                Record::ResourcesDropped,
                Record::Note("raised"),
            ],
            "the observer must see the view raised while it still runs"
        );
        assert!(stop_view.is_stopped());
    }

    #[test]
    fn test_a_termination_leaves_the_stop_view_down() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let delivery = builder.build();

        assert!(delivery.send(completed()).is_break());

        assert_eq!(
            records(&log),
            [
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
        assert!(!stop_view.is_stopped());
    }

    #[test]
    fn test_a_delivery_stop_from_on_termination_raises_the_stop_view() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let stop_view_of_observer = stop_view.clone();
        let log_of_observer = log.clone();
        let delivery = builder
            .on_termination(move |delivery, _termination| {
                assert!(
                    !stop_view_of_observer.is_stopped(),
                    "the termination itself must not raise the view"
                );
                // The termination has already stopped the delivery: the stop raises the view
                // all the same.
                delivery.stop_handle().stop();
                if stop_view_of_observer.is_stopped() {
                    record(&log_of_observer, Record::Note("raised"));
                }
            })
            .build();

        assert!(delivery.send(completed()).is_break());

        assert_eq!(
            records(&log)[..2],
            [
                Record::Termination(Termination::Completed),
                Record::Note("raised"),
            ]
        );
        assert!(stop_view.is_stopped());
    }

    #[test]
    fn test_a_stop_view_sends_while_the_delivery_runs() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let _delivery = builder.build();
        let stop_view = stop_view.take();
        let sent = new_log();
        let mut target = target(&sent, |_| {});

        assert!(stop_view.send(&mut target, || 1).is_continue());
        assert!(
            stop_view
                .send_each(&mut target, [2, 3], |n| n)
                .is_continue()
        );
        stop_view.terminate(target, || Termination::Completed);

        assert_eq!(
            records(&sent),
            [
                Record::Next(1),
                Record::Next(2),
                Record::Next(3),
                Record::Termination(Termination::Completed),
            ]
        );
    }

    #[test]
    fn test_a_stopped_view_runs_no_user_code() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let delivery = builder.build();
        let stop_view = stop_view.take();
        delivery.stop_handle().stop();
        let sent = new_log();
        let mut target = target(&sent, |_| {});

        assert!(stop_view.send(&mut target, || unreachable!()).is_break());
        let flow = stop_view.send_each(&mut target, [1], |_| -> i32 { unreachable!() });
        assert!(flow.is_break());
        stop_view.terminate(target, || unreachable!());

        assert!(records(&sent).is_empty());
    }

    /// The user's code that makes a value or a termination may dispose the subscription, which
    /// drops what it made unsent.
    #[test]
    fn test_a_stop_while_the_event_is_made_drops_it_unsent() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let delivery = builder.build();
        let stop_view = stop_view.take();
        let sent = new_log();
        let mut target = target(&sent, |_| {});

        let flow = stop_view.send(&mut target, || {
            delivery.stop_handle().stop();
            1
        });
        assert!(flow.is_break());
        stop_view.terminate(target, || Termination::Completed);

        assert!(records(&sent).is_empty());
    }

    /// An emission that disposes the subscription ends a `send_each` before it makes the next
    /// value.
    #[test]
    fn test_a_stop_from_an_emission_ends_send_each() {
        let log = new_log();
        let builder = builder(&log);
        let stop_view = builder.stop_view();
        let delivery = builder.build();
        let stop_view = stop_view.take();
        let stop = delivery.stop_handle();
        let sent = new_log();
        let mut target = target(&sent, move |value| {
            if value == 2 {
                stop.stop();
            }
        });
        let mut made = Vec::new();

        let flow = stop_view.send_each(&mut target, [1, 2, 3], |n| {
            made.push(n);
            n
        });

        assert!(flow.is_break());
        assert_eq!(made, [1, 2]);
        assert_eq!(values(&sent), [1, 2]);
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
                    // so the answer is already `Break` — and nothing can be queued after it, even
                    // before it is delivered.
                    assert!(delivery.send(completed()).is_break());
                    assert!(delivery.send(next(10)).is_break());
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_break());
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
                flow: Flow::Continue(())
            }
        );
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Ran {
                result: 11,
                flow: Flow::Continue(())
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
                        flow: Flow::Continue(())
                    }
                );
            })
            .build();

        assert!(delivery.send(next_batch([1, 2, 3])).is_continue());
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Ran {
                result: 6,
                flow: Flow::Continue(())
            }
        );
    }

    #[test]
    fn test_update_behind_a_queued_termination_does_not_run() {
        let log = new_log();
        let log_of_observer = log.clone();
        let delivery = builder(&log)
            .on_next(move |delivery, number| {
                if number == 1 {
                    assert!(delivery.send(completed()).is_break());
                    // The resources live until the termination is delivered, but the stream's end
                    // is decided: the update does not run, and what it captured is dropped at
                    // once, before the value queued ahead of the termination is delivered.
                    let probe = Value::new(10).on_drop(note_and_reenter(
                        &DeliveryHandle::of(delivery),
                        &log_of_observer,
                        "update dropped",
                    ));
                    let updated: Updated<i32> =
                        delivery.update(move |_resources| -> TestOutcome<i32> {
                            drop(probe); // Never runs: it is what makes the update own the probe.
                            panic!("the update must not run behind a queued termination");
                        });
                    assert_eq!(updated, Updated::Ended);
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2])).is_break());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Note("update dropped"),
                Record::Next(2),
                Record::Termination(Termination::Completed),
                Record::ObserverDropped,
                Record::ResourcesDropped,
            ]
        );
    }

    #[test]
    fn test_update_after_a_termination_answers_ended() {
        let log = new_log();
        let delivery = builder(&log).build();

        assert!(delivery.send(completed()).is_break());
        assert_eq!(
            delivery.update(|resources| UpdateOutcome::new(resources.model)),
            Updated::Ended
        );
    }

    #[test]
    fn test_an_update_of_a_stopped_delivery_does_not_run_and_is_dropped_outside_the_lock() {
        let log = new_log();
        let delivery = builder(&log).build();
        delivery.stop_handle().stop();

        let probe = Value::new(0).on_drop(note_and_reenter(
            &DeliveryHandle::of(&delivery),
            &log,
            "update dropped",
        ));
        let result: Updated<()> = delivery.update(move |_resources| -> TestOutcome<()> {
            drop(probe); // Never runs: it is what makes the update own the probe.
            panic!("the update must not run once the delivery has stopped");
        });

        assert_eq!(result, Updated::Ended);
        // The update releases the observer the stop left parked, before the update itself.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("update dropped"),
            ],
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
                flow: Flow::Continue(())
            }
        );
        assert_eq!(
            delivery.update(|_resources| {
                UpdateOutcome::empty()
                    .with_events(EventBatch::NextBatch(vec![Value::new(2), Value::new(3)]))
            }),
            Updated::Ran {
                result: (),
                flow: Flow::Continue(())
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
                flow: Flow::Break(())
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
                flow: Flow::Continue(())
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
                flow: Flow::Break(())
            }
        );
        assert_eq!(
            delivery.update(|_resources| UpdateOutcome::empty()),
            Updated::Ended
        );
        assert!(delivery.send(next(1)).is_break());
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
                            flow: Flow::Continue(())
                        }
                    );
                }
            })
            .build();

        assert!(delivery.send(next_batch([1, 2])).is_continue());
        assert_eq!(values(&log), [1, 2, 10]);
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
                flow: Flow::Break(())
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
                flow: Flow::Continue(())
            }
        );
        assert_eq!(
            update(),
            Updated::Ran {
                result: 2,
                flow: Flow::Continue(())
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
        delivery.stop_handle().stop();
        assert!(clone.send(next(2)).is_break());
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

    #[test]
    fn test_dropping_the_last_handle_leaves_resources_owned_by_the_stop_handle() {
        let log = new_log();
        let delivery = builder(&log).build();
        let stop = delivery.stop_handle();

        drop(delivery);
        assert_eq!(records(&log), [Record::ObserverDropped]);

        stop.stop();
        assert_eq!(
            records(&log),
            [Record::ObserverDropped, Record::ResourcesDropped]
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
        assert!(delivery.send(next(4)).is_break());
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_during_a_batch_drops_the_remaining_values_outside_the_lock() {
        let log = new_log();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let handle_of_observer = handle.clone();
        let log_of_observer = log.clone();
        let delivery = builder
            .on_next(move |delivery, number| {
                if number == 2 {
                    // Value 3 is in the loop's batch; value 4 and the termination stay queued.
                    let queued = Value::new(4).on_drop(note_and_reenter(
                        &handle_of_observer,
                        &log_of_observer,
                        "4 dropped",
                    ));
                    assert!(
                        delivery
                            .send(EventBatch::NextAndTermination(
                                queued,
                                Termination::Completed
                            ))
                            .is_break()
                    );
                    resume_unwind(Box::new(()));
                }
            })
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();
        delivery
            .update(|resources| {
                resources.on_drop(note_and_reenter(&handle, &log, "resources dropped"));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");

        let undelivered = Value::new(3).on_drop(note_and_reenter(&handle, &log, "3 dropped"));
        let result = catch_unwind(AssertUnwindSafe(|| {
            delivery.send(EventBatch::NextBatch(vec![
                Value::new(1),
                Value::new(2),
                undelivered,
            ]))
        }));
        assert!(result.is_err());
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Next(2),
                // The guard stops the delivery, dropping what the queue held and the resources.
                Record::Note("4 dropped"),
                Record::ResourcesDropped,
                Record::Note("resources dropped"),
                // It then drops the observer, and the unwind drops the batch the loop held.
                Record::ObserverDropped,
                Record::Note("observer dropped"),
                Record::Note("3 dropped"),
            ]
        );
        assert!(delivery.send(next(5)).is_break());
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
        assert!(delivery.send(next(1)).is_break());
    }

    /// A callback that notes the drop of whatever owns it, then unwinds out of that drop.
    #[cfg(panic = "unwind")]
    fn note_and_panic(log: &Log, text: &'static str) -> DropCallback {
        let log = log.clone();
        Box::new(move || {
            record(&log, Record::Note(text));
            resume_unwind(Box::new(()));
        })
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_dropping_a_rejected_value_still_releases_a_parked_observer() {
        let log = new_log();
        let delivery = builder(&log).build();
        delivery.stop_handle().stop();

        let rejected = Value::new(1).on_drop(note_and_panic(&log, "1 dropped"));
        let result = catch_unwind(AssertUnwindSafe(|| {
            delivery.send(EventBatch::Next(rejected))
        }));
        assert!(result.is_err());

        // The send claimed the parked observer, so nothing else would release it while the
        // delivery lives on: it is released before the rejected value unwinds.
        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("1 dropped"),
            ]
        );
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_dropping_a_rejected_update_still_releases_a_parked_observer() {
        let log = new_log();
        let delivery = builder(&log).build();
        delivery.stop_handle().stop();

        let captured = Value::new(1).on_drop(note_and_panic(&log, "update dropped"));
        let result = catch_unwind(AssertUnwindSafe(|| {
            delivery.update(move |_resources| {
                drop(captured);
                UpdateOutcome::empty()
            })
        }));
        assert!(result.is_err());

        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("update dropped"),
            ]
        );
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_dropping_a_parked_observer_still_drops_the_rejected_value() {
        let log = new_log();
        let delivery = builder(&log)
            .on_observer_drop(note_and_panic(&log, "observer dropped"))
            .build();
        delivery.stop_handle().stop();

        let rejected = Value::new(1).on_drop(note(&log, "1 dropped"));
        let result = catch_unwind(AssertUnwindSafe(|| {
            delivery.send(EventBatch::Next(rejected))
        }));
        assert!(result.is_err());

        assert_eq!(
            records(&log),
            [
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
                Record::Note("1 dropped"),
            ]
        );
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_a_panic_dropping_the_resources_of_an_observer_that_stops_keeps_the_drop_order() {
        let log = new_log();
        let delivery = builder(&log).stop_after(2).build();
        delivery
            .update(|resources| {
                resources.on_drop(note_and_panic(&log, "resources dropped"));
                UpdateOutcome::empty()
            })
            .result()
            .expect("the delivery is idle");

        let result = catch_unwind(AssertUnwindSafe(|| {
            delivery.send(EventBatch::NextBatch(vec![
                Value::new(1),
                Value::new(2),
                Value::new(3).on_drop(note(&log, "3 dropped")),
            ]))
        }));
        assert!(result.is_err());

        // The observer, then 3, which the loop took with 2, are dropped by the unwind out of the
        // stop, in the order the stop would have dropped them.
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Next(2),
                Record::ResourcesDropped,
                Record::Note("resources dropped"),
                Record::ObserverDropped,
                Record::Note("3 dropped"),
            ]
        );
    }

    // MARK: - Concurrency

    #[test]
    fn test_a_concurrent_stop_during_a_batch_suppresses_the_remaining_events() {
        const TIMEOUT: Duration = Duration::from_secs(5);
        let log = new_log();
        let (entered, wait_entered) = mpsc::channel();
        let (resume, wait_resume) = mpsc::channel();
        let mut builder = builder(&log);
        let handle = builder.handle();
        let delivery = builder
            .on_next(move |_delivery, number| {
                if number == 2 {
                    entered.send(()).expect("the test is waiting for value 2");
                    wait_resume
                        .recv_timeout(TIMEOUT)
                        .expect("the test resumes delivery after stopping it");
                }
            })
            .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
            .build();
        let stop = delivery.stop_handle();
        let undelivered = Value::new(3).on_drop(note_and_reenter(&handle, &log, "3 dropped"));

        std::thread::scope(|scope| {
            let sending = delivery.clone();
            let worker = scope.spawn(move || {
                sending.send(EventBatch::NextBatch(vec![
                    Value::new(1),
                    Value::new(2),
                    undelivered,
                ]))
            });
            wait_entered
                .recv_timeout(TIMEOUT)
                .expect("delivery reaches value 2");
            let queued = Value::new(4).on_drop(note_and_reenter(&handle, &log, "4 dropped"));
            assert!(
                delivery
                    .send(EventBatch::NextAndTermination(
                        queued,
                        Termination::Completed
                    ))
                    .is_break()
            );

            stop.stop();
            assert_eq!(
                records(&log),
                [
                    Record::Next(1),
                    Record::Next(2),
                    Record::Note("4 dropped"),
                    Record::ResourcesDropped,
                ]
            );

            resume.send(()).expect("delivery is waiting to resume");
            assert!(worker.join().expect("delivery must not panic").is_break());
        });
        assert_eq!(
            records(&log),
            [
                Record::Next(1),
                Record::Next(2),
                Record::Note("4 dropped"),
                Record::ResourcesDropped,
                Record::ObserverDropped,
                Record::Note("observer dropped"),
                Record::Note("3 dropped"),
            ]
        );
        assert!(delivery.send(next(5)).is_break());
    }

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
                    !delivering_of_observer.swap(true, Ordering::SeqCst),
                    "two values must never be delivered at the same time"
                );
                std::thread::yield_now();
                assert!(delivering_of_observer.swap(false, Ordering::SeqCst));
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

        // Interleaving between producers is free, but each producer's values stay in order.
        let mut delivered = values(&log);
        for thread in 0..THREADS {
            assert_eq!(
                delivered
                    .iter()
                    .copied()
                    .filter(|value| *value / VALUES_PER_THREAD == thread)
                    .collect::<Vec<_>>(),
                (thread * VALUES_PER_THREAD..(thread + 1) * VALUES_PER_THREAD).collect::<Vec<_>>()
            );
        }

        // Whichever thread delivered them, every value arrives exactly once.
        delivered.sort_unstable();
        assert_eq!(
            delivered,
            (0..THREADS * VALUES_PER_THREAD).collect::<Vec<_>>()
        );
    }
}
