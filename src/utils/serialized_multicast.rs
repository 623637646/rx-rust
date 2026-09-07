//! A multicast built on a [`SerializedDelivery`] that never terminates.
//!
//! [`SerializedMulticast`] is what a subject is made of: it owns the observers, the termination,
//! the ids that order the observers, and whatever else its host needs (`R`). Everything the host
//! must read before it decides what to emit is therefore guarded by **one lock** — the delivery's
//! — so reading the termination, changing the host's own state and queueing the events it produces
//! is a single atomic step. A host that keeps state of its own next to this one loses that: every
//! check-then-act pair across two locks is a window another thread, or a re-entrant callback, can
//! slip through.
//!
//! The multicast's termination travels as an `Action::Terminate`, an ordinary value of the
//! delivery. **Nothing here may send an [`EventBatch::Termination`] to that delivery**: that would
//! drop the subscribers and the resources, silently killing the multicast. Keeping the delivery
//! alive is what lets an observer that arrives after the termination still be notified with it,
//! from the resources.
//!
//! Recording the termination and queueing the action that delivers it is one step, and so is
//! admitting a subscription — nothing can ever be queued behind the `Action::Terminate`, so the
//! subscribers need no notion of termination of their own. The multicast is consequently
//! terminated as soon as the termination is *queued*, not when it reaches the observers.
//!
//! The observers live inside `Subscribers`, which the delivery loop owns while it delivers, so
//! subscribing, unsubscribing and terminating all travel as `Action`s and touch the observers
//! only outside the lock. Unsubscribing is the one that must take effect at once: the disposal
//! writes a flag the subscribers check before every notification, and the queued `Action::Prune`
//! only releases the observer afterwards.
//!
//! # Replaying to a newcomer
//!
//! A host that replays something to a joining observer — the current value, a buffer, the last
//! value of a completed subject — hands it to [`Admission::Join`] under the lock, and the values
//! travel *inside* the `Action::Add`. They are delivered by the delivery loop, right before the
//! entry joins, and therefore in the same serialized stream as everything else: the snapshot the
//! host took cannot miss a value forwarded after it, nor repeat one forwarded before it. The
//! observer has moved into the entry by then, so this is also the only place it can be notified
//! without racing the loop that may already be feeding the other observers.
//!
//! The replay is consequently *not* guaranteed to happen before `subscribe` returns: it does when
//! the delivery is idle, since the action is then applied on the subscribing thread, but a
//! subscription made while a delivery is running is served by that delivery instead.

use crate::disposable::Disposable;
use crate::observer::{Observer, Termination, boxed_observer::BoxedObserver};
use crate::utils::id_generator::{Id, IdGenerator};
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::{DeliveryStopped, SerializedDelivery, UpdateOutcome};
use crate::utils::types::{MaybeSend, MutableBool, MutableBoolHelper, Shared};
use educe::Educe;

/// A shared, serialized delivery of events to many observers, guarding the host's state with it.
///
/// `R` is whatever the host owns besides the observers: the current value of a behavior subject,
/// the buffer of a replay subject, `()` when it owns nothing.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SerializedMulticast<'or, T, E, R = ()>(Delivery<'or, T, E, R>);

/// Serializes every action against every value, and guards the whole state as its resources. Its
/// termination is never sent: see the module documentation.
type Delivery<'or, T, E, R> =
    SerializedDelivery<Action<'or, T, E>, E, Subscribers<'or, T, E>, Resources<E, R>>;

/// Everything the multicast owns besides its observers, guarded by the delivery's lock.
#[derive(Educe)]
#[educe(Debug)]
struct Resources<E, R> {
    /// Recorded when the `Action::Terminate` is queued: that is what terminates the multicast.
    termination: Option<Termination<E>>,
    /// Handed out in subscription order, so that the entries stay sorted by it.
    ids: IdGenerator,
    /// The host's own state, read and written under this very lock.
    host: R,
}

/// What the host decided, under the lock, for an observer that wants to join.
#[derive(Educe)]
#[educe(Debug)]
pub enum Admission<T, E> {
    /// Deliver these values to the newcomer, then let it join the multicast. Use an empty [`Vec`],
    /// which allocates nothing, when there is nothing to replay.
    Join(Vec<T>),
    /// Do not join: deliver these values and then this termination, to the newcomer alone.
    Terminated(Vec<T>, Termination<E>),
}

/// What [`SerializedMulticast::subscribe_with`] did under the lock, for the observer waiting
/// outside it.
#[derive(Educe)]
#[educe(Debug)]
enum Admitted<T, E> {
    /// The entry was queued with this id, carrying the observer and its replay with it.
    Added(Id),
    /// The observer stayed behind, to be notified with these events.
    Terminated(Vec<T>, Termination<E>),
}

impl<'or, T, E, R> SerializedMulticast<'or, T, E, R> {
    /// Starts with no observer, no termination, and the host's state parked in the resources.
    pub fn idle(host: R) -> Self {
        Self(SerializedDelivery::idle(
            Subscribers {
                entries: Vec::new(),
            },
            Resources {
                termination: None,
                ids: IdGenerator::default(),
                host,
            },
        ))
    }
}

impl<'or, T, E, R> SerializedMulticast<'or, T, E, R>
where
    T: Clone,
    E: Clone,
{
    /// The termination, once one has been queued.
    ///
    /// The resources are gone once the delivery stopped, which only an observer's panic does: the
    /// multicast is then dead, and reports no termination.
    pub fn terminated(&self) -> Option<Termination<E>> {
        self.0
            .update(|resources| UpdateOutcome::new(resources.termination.clone()))
            .unwrap_or(None)
    }

    /// Reads the host's state and the termination together, under the lock.
    ///
    /// `read` must not notify anyone and must not drop a value that can re-enter this multicast:
    /// it runs under the lock. Returns [`DeliveryStopped`], without running `read`, once the
    /// delivery has stopped.
    pub fn read<Out>(
        &self,
        read: impl FnOnce(&R, Option<&Termination<E>>) -> Out,
    ) -> Result<Out, DeliveryStopped> {
        self.0.update(|resources| {
            UpdateOutcome::new(read(&resources.host, resources.termination.as_ref()))
        })
    }

    /// Updates the host's state and queues the events that update produced, under one lock.
    ///
    /// `update` sees the termination, so it can decide whether it may emit at all, and describes
    /// its outcome with an [`UpdateOutcome`] over the *host's* events: a
    /// [`Termination`](EventBatch::Termination) in that batch is what terminates the multicast,
    /// recorded here as the action carrying it is queued. A host that emits after the termination
    /// was queued is a bug — check the termination first, and hand the rejected event to
    /// [`UpdateOutcome::with_drop_outside`].
    ///
    /// `update` must not notify anyone and must not drop a value that can re-enter this multicast:
    /// it runs under the lock. Returns [`DeliveryStopped`], without running `update`, once the
    /// delivery has stopped.
    pub fn update<Out, DO, const EVENTS_DECIDED: bool>(
        &self,
        update: impl FnOnce(
            &mut R,
            Option<&Termination<E>>,
        ) -> UpdateOutcome<T, E, Out, DO, EVENTS_DECIDED>,
    ) -> Result<Out, DeliveryStopped> {
        self.0.update(|resources| {
            let (events, drop_outside, result) =
                update(&mut resources.host, resources.termination.as_ref()).into_parts();
            // The host's decision is turned into actions under the same lock that recorded the
            // termination, so nothing can be queued between the two.
            let outcome = UpdateOutcome::new(result).with_drop_outside(drop_outside);
            match events {
                Some(events) => {
                    outcome.with_events(into_actions(events, &mut resources.termination))
                }
                None => outcome.without_events(),
            }
        })
    }

    /// Queues `events` for every observer, dropping them outside the lock once terminated.
    ///
    /// Returns whether they were queued. This is [`Self::update`] for a host that reads nothing
    /// and decides nothing.
    pub fn send(&self, events: EventBatch<T, E>) -> bool {
        self.update(|_, terminated| {
            if terminated.is_some() {
                return UpdateOutcome::new(false)
                    .with_drop_outside(events)
                    .without_events();
            }
            UpdateOutcome::new(true)
                .without_drop_outside()
                .with_events(events)
        })
        .unwrap_or(false)
    }

    /// Subscribes `observer`, replaying nothing and terminating it at once when already terminated.
    pub fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Option<MulticastDisposal<'or, T, E, R>> {
        self.subscribe_with(observer, |_, terminated| match terminated {
            Some(termination) => Admission::Terminated(Vec::new(), termination.clone()),
            None => Admission::Join(Vec::new()),
        })
    }

    /// Subscribes `observer`, letting the host decide what it observes first, under the lock.
    ///
    /// Reading the host's state, handing out the id and queueing the entry are one step, so the
    /// values `admit` snapshots are exactly the ones the newcomer missed: see the module
    /// documentation. `admit` runs under the lock and must not notify anyone.
    ///
    /// Returns the disposal of the subscription, or [`None`] when the observer did not join: it
    /// has then already been notified, outside the lock.
    pub fn subscribe_with(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
        admit: impl FnOnce(&mut R, Option<&Termination<E>>) -> Admission<T, E>,
    ) -> Option<MulticastDisposal<'or, T, E, R>> {
        let disposed = Shared::new(MutableBool::new(false));
        // The observer travels with its entry, and stays here when there is no entry to join.
        let mut observer = Some(observer);
        let admitted = self.0.update(|resources| {
            match admit(&mut resources.host, resources.termination.as_ref()) {
                Admission::Terminated(values, termination) => {
                    UpdateOutcome::new(Admitted::Terminated(values, termination)).without_events()
                }
                Admission::Join(replay) => {
                    let id = resources.ids.next_id();
                    let entry = Entry {
                        id,
                        disposed: disposed.clone(),
                        observer: BoxedObserver::new(
                            observer.take().expect("the update runs at most once"),
                        ),
                    };
                    UpdateOutcome::new(Admitted::Added(id))
                        .with_next_event(Action::Add { entry, replay })
                }
            }
        });
        match admitted {
            Ok(Admitted::Added(id)) => Some(MulticastDisposal {
                delivery: self.0,
                disposed,
                id,
            }),
            Ok(Admitted::Terminated(values, termination)) => {
                let mut observer = observer.take().expect("the update left the observer here");
                for value in values {
                    observer.on_next(value);
                }
                observer.on_termination(termination);
                None
            }
            Err(DeliveryStopped) => {
                // The delivery only stops once an observer panicked, which kills the multicast:
                // the termination went with the resources, so this observer is dropped here
                // instead, outside the lock.
                debug_assert!(false, "the multicast is dead because an observer panicked");
                None
            }
        }
    }
}

/// Translates the host's events into actions, recording the termination as it is queued.
fn into_actions<'or, T, E>(
    events: EventBatch<T, E>,
    termination: &mut Option<Termination<E>>,
) -> EventBatch<Action<'or, T, E>, E>
where
    E: Clone,
{
    let mut record = |queued: Termination<E>| {
        debug_assert!(
            termination.is_none(),
            "a host must not emit once the termination was queued"
        );
        *termination = Some(queued.clone());
        // Never an `EventBatch::Termination`: the delivery must stay alive, see the module
        // documentation.
        Action::Terminate(queued)
    };
    match events {
        EventBatch::Next(value) => EventBatch::Next(Action::Forward(value)),
        EventBatch::Termination(termination) => EventBatch::Next(record(termination)),
        EventBatch::NextAndTermination(value, termination) => {
            EventBatch::NextBatch(vec![Action::Forward(value), record(termination)])
        }
        EventBatch::NextBatch(values) => {
            EventBatch::NextBatch(values.into_iter().map(Action::Forward).collect())
        }
        EventBatch::NextBatchAndTermination(values, termination) => {
            let mut actions: Vec<_> = values.into_iter().map(Action::Forward).collect();
            actions.push(record(termination));
            EventBatch::NextBatch(actions)
        }
    }
}

/// One subscribed observer.
#[derive(Educe)]
#[educe(Debug)]
struct Entry<'or, T, E> {
    /// Identifies the entry before it has been added, so a subscription can be disposed while its
    /// `Action::Add` is still queued.
    id: Id,
    /// Written by the disposal, read before every notification.
    disposed: Shared<MutableBool>,
    observer: BoxedObserver<'or, T, E>,
}

/// Everything that reaches the observers, serialized by the delivery and applied outside its lock.
#[derive(Educe)]
#[educe(Debug)]
enum Action<'or, T, E> {
    /// Sends a value to every entry that is still subscribed.
    Forward(T),
    /// Replays `replay` to the entry's observer, then adds the entry.
    Add {
        entry: Entry<'or, T, E>,
        replay: Vec<T>,
    },
    /// Removes the entry with this id, releasing its observer.
    Prune(Id),
    /// Terminates every entry. An observer that subscribes afterwards is terminated by
    /// [`SerializedMulticast::subscribe_with`] instead.
    Terminate(Termination<E>),
}

/// Owns the observers, so that they are fed outside the lock that serializes the actions.
#[derive(Educe)]
#[educe(Debug)]
struct Subscribers<'or, T, E> {
    /// Sorted by id, which is handed out in subscription order: notifications follow that order,
    /// and an id is found by binary search.
    entries: Vec<Entry<'or, T, E>>,
}

impl<'or, T, E> Observer<Action<'or, T, E>, E> for Subscribers<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, action: Action<'or, T, E>) {
        match action {
            Action::Forward(value) => self.forward(value),
            Action::Add { entry, replay } => self.add(entry, replay),
            Action::Prune(id) => self.prune(id),
            Action::Terminate(termination) => self.terminate(termination),
        }
    }

    fn on_termination(self, _: Termination<E>) {
        debug_assert!(
            false,
            "the multicast's delivery never terminates: see the module documentation"
        );
    }
}

impl<'or, T, E> Subscribers<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn forward(&mut self, value: T) {
        for entry in &mut self.entries {
            if entry.disposed.read() {
                continue; // Unsubscribed, possibly during this very dispatch.
            }
            entry.observer.on_next(value.clone());
        }
    }

    fn add(&mut self, mut entry: Entry<'or, T, E>, replay: Vec<T>) {
        // The replay is delivered here, where it is serialized with everything else: the values
        // the host snapshotted are exactly the ones queued before this action.
        for value in replay {
            if entry.disposed.read() {
                return; // Unsubscribed, possibly during this very replay.
            }
            entry.observer.on_next(value);
        }
        if entry.disposed.read() {
            return; // Unsubscribed before it was ever added.
        }
        debug_assert!(
            self.entries.last().is_none_or(|last| last.id < entry.id),
            "the ids are handed out in subscription order"
        );
        self.entries.push(entry);
    }

    fn prune(&mut self, id: Id) {
        if let Ok(index) = self.entries.binary_search_by_key(&id, |entry| entry.id) {
            self.entries.remove(index); // Releases the observer here, outside the lock
        }
    }

    fn terminate(&mut self, termination: Termination<E>) {
        // Nothing is queued behind the termination, so emptying the entries here is final.
        for entry in std::mem::take(&mut self.entries) {
            if entry.disposed.read() {
                continue; // Unsubscribed, possibly during this very dispatch.
            }
            entry.observer.on_termination(termination.clone());
        }
    }
}

/// Unsubscribes one observer from a [`SerializedMulticast`].
pub struct MulticastDisposal<'or, T, E, R> {
    delivery: Delivery<'or, T, E, R>,
    disposed: Shared<MutableBool>,
    id: Id,
}

impl<T, E, R> Disposable for MulticastDisposal<'_, T, E, R>
where
    T: Clone,
    E: Clone,
{
    fn dispose(self) {
        // The flag is what stops the events; the action only releases the observer afterwards.
        self.disposed.write(true);
        self.delivery.send(EventBatch::Next(Action::Prune(self.id)));
    }
}
