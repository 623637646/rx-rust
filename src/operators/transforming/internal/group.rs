//! What a buffer or a window operator does with its groups — the bundles of a buffer, the windows
//! of a window — as they open, take an item, close and end (decision 0014).
//!
//! The four ways the groups are opened and closed — a count and a skip
//! ([`count`](super::count)), a time and a shift ([`time_shift`](super::time_shift)), a time or a
//! count ([`time_or_count`](super::time_or_count)) and a boundary
//! ([`boundary`](super::boundary)) — are each one state machine, generic over the [`Groups`] it
//! drives: [`Bundles`](super::bundle::Bundles) for the buffers,
//! [`Windows`](super::window::Windows) for the windows. This module holds what the machines share
//! — the trait, the [`Events`] its hooks emit to under a lock and [`update_groups`] — and each
//! kind of group lives in a module of its own.
//!
//! The two differ in where the groups live. A bundle is a `Vec`, and putting an item in one runs
//! no user code, so the bundles live in the state the machine keeps under its lock, and the only
//! event is a finished bundle. A window is a subject whose subscriber is user code, so the windows
//! live outside the lock, in the [`DelegateObserver`](super::window::DelegateObserver), and every
//! item and every change is an event for it. Either way, an item takes the one lock the machine
//! takes anyway: a bundle's is pushed under it, a window's is queued as an event.

use crate::{
    disposable::Disposable,
    observer::{EventBatch, Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::{DropUndecided, UpdateOutcome, Updated},
        subscribe_with_context::SubscriptionContext,
    },
};

/// The groups of a buffer or window operator, and what their changes emit.
///
/// The hooks run where the machine keeps its state — under the lock of a machine that has one —
/// so they must not run user code beyond the `duplicate` of the items: what does goes into an
/// event.
///
/// Each hook emits to `observer`, borrowed so that it can send values but never terminate it: the
/// observer of a machine with no lock itself, or the [`Events`] a machine with a lock queues to
/// deliver once the lock is released. A machine with a lock opens its first group before it
/// creates its context, which nothing else can reach yet, so that one opening emits to the
/// machine's observer directly, without the lock. A hook answers the flow of the last event it
/// emitted, [`Flow::Continue`] when it emitted none, and emits nothing after a [`Flow::Break`],
/// since an observer that stopped accepts nothing more. The [`Events`] always answer [`Flow::Continue`],
/// the context applying a stop as it delivers them, so a machine with a lock ignores what the
/// hooks answer.
pub(crate) trait Groups<E> {
    /// The items the groups take.
    type Item;

    /// What the groups emit, for the machine's observer to deliver.
    type Event;

    /// Opens a new group, the newest.
    fn open(&mut self, observer: &mut impl Observer<Self::Event, E>) -> Flow;

    /// Puts the item into every open group, leaving it in `value` when there is none, for the
    /// caller to drop outside the lock: pushed into the bundles, emitted for the windows.
    fn put(
        &mut self,
        value: &mut Option<Self::Item>,
        observer: &mut impl Observer<Self::Event, E>,
    ) -> Flow;

    /// Closes the oldest open group.
    fn close_oldest(&mut self, observer: &mut impl Observer<Self::Event, E>) -> Flow;

    /// Closes the oldest open group, then opens the next: a window is completed before the next
    /// one is emitted. What a boundary, a full group or a tick of a machine with one group at a
    /// time does.
    #[inline]
    fn rotate(&mut self, observer: &mut impl Observer<Self::Event, E>) -> Flow {
        self.close_oldest(observer)?;
        self.open(observer)
    }

    /// Ends the groups still open as the stream ends with `termination`, which the machine then
    /// delivers unless the observer stopped on what they emitted.
    fn end(
        &mut self,
        termination: &Termination<E>,
        observer: &mut impl Observer<Self::Event, E>,
    ) -> Flow;
}

/// Runs `change` on the model of `context` under its lock, then queues what the groups emitted:
/// every change a machine with a lock makes to its groups but their end.
pub(crate) fn update_groups<M, E, Ev, OR, MD, D, R>(
    context: &SubscriptionContext<M, Ev, E, OR, MD, D>,
    change: impl FnOnce(&mut MD, &mut Events<Ev>) -> R,
) -> Updated<R>
where
    M: ThreadMode,
    OR: Observer<Ev, E>,
    D: Disposable,
{
    context.update(|model| {
        let mut events = Events::new();
        let result = change(model, &mut events);
        events.outcome(result)
    })
}

/// The observer the groups of a machine with a lock emit to under it: their events, to queue at
/// once.
///
/// One event, the common case, is held as it is: only two or more take a `Vec`.
pub(crate) enum Events<Ev> {
    /// None.
    Zero,
    /// One.
    One(Ev),
    /// Several, in order.
    Many(Vec<Ev>),
}

/// The events reach the machine's observer once the lock is released, through the context, which
/// stops delivering them as that observer stops: the flow is not known yet, so every event answers
/// [`Flow::Continue`].
impl<Ev, E> Observer<Ev, E> for Events<Ev> {
    #[inline]
    fn on_next(&mut self, event: Ev) -> Flow {
        *self = match std::mem::replace(self, Self::Zero) {
            Self::Zero => Self::One(event),
            Self::One(first) => {
                // Room for a window's item, the completion of the oldest and the opening of the
                // next, the most a single item or tick emits, in one allocation.
                let mut events = Vec::with_capacity(4);
                events.push(first);
                events.push(event);
                Self::Many(events)
            }
            Self::Many(mut events) => {
                events.push(event);
                Self::Many(events)
            }
        };
        Flow::Continue(())
    }

    fn on_termination(self, _: Termination<E>) {
        // The hooks get their observer by reference: a machine ends its stream with `ending`.
        unreachable!("the groups never terminate their observer")
    }
}

impl<Ev> Events<Ev> {
    /// No event yet.
    #[inline]
    pub(crate) fn new() -> Self {
        Self::Zero
    }

    /// The outcome of an update answering `result` that queues these events.
    #[inline]
    fn outcome<E, R>(self, result: R) -> UpdateOutcome<Ev, E, R, DropUndecided, true> {
        let outcome = UpdateOutcome::new(result);
        match self {
            Self::Zero => outcome.without_events(),
            Self::One(event) => outcome.with_next_event(event),
            Self::Many(events) => outcome.with_events(EventBatch::NextBatch(events)),
        }
    }

    /// The outcome of an update that ends the groups, then the stream, with `termination`.
    pub(crate) fn ending<E>(
        groups: &mut impl Groups<E, Event = Ev>,
        termination: Termination<E>,
    ) -> UpdateOutcome<Ev, E, (), DropUndecided, true> {
        let mut events = Self::new();
        let _ = groups.end(&termination, &mut events);
        let outcome = UpdateOutcome::empty();
        match events {
            Self::Zero => outcome.with_termination_event(termination),
            Self::One(event) => outcome.with_next_and_termination_events(event, termination),
            Self::Many(events) => {
                outcome.with_events(EventBatch::NextBatchAndTermination(events, termination))
            }
        }
    }
}

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;
