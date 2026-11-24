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
mod tests;
