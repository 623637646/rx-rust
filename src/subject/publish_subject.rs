//! A multicast subject built on a [`SerializedDelivery`] that never terminates.
//!
//! The subject's own termination travels as a [`SubjectAction::Terminate`], an ordinary value of
//! the delivery. **Nothing here may send an [`EventBatch::Termination`] to that delivery**: that
//! would drop the delegate observer and the resources, silently killing the subject. Keeping the
//! delivery alive is what lets an observer that subscribes after the termination still be notified
//! with it, from the resources.
//!
//! The delivery's lock is the subject's only lock: the termination and the id of the next entry
//! are its resources, read and written under the very lock that orders the actions. Recording the
//! termination and queueing the action that delivers it is therefore one atomic step, and so is
//! admitting a subscription — nothing can ever be queued behind the [`SubjectAction::Terminate`],
//! so the delegate needs no notion of termination of its own. The subject is consequently
//! terminated as soon as [`Observer::on_termination`] is *called*, not when the termination
//! reaches the observers.
//!
//! The observers live inside [`DelegateObserver`], which the delivery loop owns while it delivers,
//! so subscribing, unsubscribing and terminating all travel as [`SubjectAction`]s and touch the
//! observers only outside the lock. Unsubscribing is the one that must take effect at once: the
//! disposal writes a flag the delegate checks before every notification, and the queued
//! [`SubjectAction::Prune`] only releases the observer afterwards.

use super::Subject;
use crate::delegate_disposal;
use crate::disposable::option_disposal::OptionDisposal;
use crate::disposable::{Disposable, DisposableExt};
use crate::observable::Subscription;
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::{DeliveryStopped, SerializedDelivery, UpdateOutcome};
use crate::utils::types::{MaybeSend, MutableBool, MutableBoolHelper, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Basic multicast subject that forwards events to all observers.
///
/// Observers are notified in subscription order, and an observer that unsubscribes does not
/// disturb the order of the others.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E>(SubjectDelivery<'or, T, E>);

/// Serializes every action against every value, and guards the subject's whole state as its
/// resources. Its termination is never sent: see the module documentation.
type SubjectDelivery<'or, T, E> = SerializedDelivery<
    SubjectAction<'or, T, E>,
    E,
    DelegateObserver<'or, T, E>,
    SubjectResources<E>,
>;

/// Everything the subject owns besides its observers, guarded by the delivery's lock.
#[derive(Educe)]
#[educe(Debug)]
struct SubjectResources<E> {
    /// Recorded when the [`SubjectAction::Terminate`] is queued: that is what terminates the
    /// subject.
    termination: Option<Termination<E>>,
    /// Handed out in subscription order, so that the delegate's entries stay sorted by it.
    next_id: u64,
}

/// What [`Observable::subscribe`] decided under the lock, for the observer waiting outside it.
#[derive(Educe)]
#[educe(Debug)]
enum Admission<E> {
    /// The entry was queued with this id, carrying the observer with it.
    Added(u64),
    /// The subject had already terminated; the observer stayed behind, to be notified with this.
    Terminated(Termination<E>),
}

impl<'or, T, E> PublishSubject<'or, T, E> {
    pub fn new() -> Self {
        let delegate = DelegateObserver {
            entries: Vec::new(),
        };
        Self(SerializedDelivery::idle(
            delegate,
            SubjectResources {
                termination: None,
                next_id: 0,
            },
        ))
    }
}

impl<T, E> Default for PublishSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<PublishSubjectDisposal<'or, T, E>>,
);

impl<'or, T, E> Observable<'or, T, E> for PublishSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let disposed = Shared::new(MutableBool::new(false));
        // The observer travels with its entry, and stays here when there is no entry to join.
        let mut observer = Some(observer);
        // Reading the termination, handing out the id and queueing the entry are one step under
        // one lock.
        let admission = self.0.update_and_send(|resources| {
            if let Some(termination) = resources.termination.clone() {
                return UpdateOutcome::new(Admission::Terminated(termination)).without_events();
            }
            let id = resources.next_id;
            resources.next_id += 1;
            let entry = Entry {
                id,
                disposed: disposed.clone(),
                observer: BoxedObserver::new(
                    observer.take().expect("the update runs at most once"),
                ),
            };
            UpdateOutcome::new(Admission::Added(id)).with_next_event(SubjectAction::Add(entry))
        });
        match admission {
            Ok(Admission::Added(id)) => OptionDisposal::some(PublishSubjectDisposal {
                delivery: self.0,
                disposed,
                id,
            })
            .into_subscription(),
            Ok(Admission::Terminated(termination)) => {
                let observer = observer.take().expect("the update left the observer here");
                observer.on_termination(termination);
                OptionDisposal::none().into_subscription()
            }
            Err(DeliveryStopped) => {
                // The delivery only stops once an observer panicked, which kills the subject: the
                // termination went with the resources, so this observer is dropped here instead,
                // outside the lock.
                debug_assert!(false, "the subject is dead because an observer panicked");
                OptionDisposal::none().into_subscription()
            }
        }
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        // One step under one lock, so a value can never be queued behind the termination. A value
        // that arrives after it is dropped outside the lock.
        let _ = self.0.update_and_send(|resources| {
            if resources.termination.is_some() {
                return UpdateOutcome::empty()
                    .with_drop_outside(value)
                    .without_events();
            }
            UpdateOutcome::empty()
                .without_drop_outside()
                .with_next_event(SubjectAction::Forward(value))
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        // One step under one lock, so only the first termination is ever queued, and nothing joins
        // the subject after it.
        let _ = self.0.update_and_send(|resources| {
            if resources.termination.is_some() {
                // Already terminated: this termination is dropped outside the lock.
                return UpdateOutcome::empty()
                    .with_drop_outside(termination)
                    .without_events();
            }
            resources.termination = Some(termination.clone());
            UpdateOutcome::empty()
                .without_drop_outside()
                .with_next_event(SubjectAction::Terminate(termination))
        });
    }
}

impl<'or, T, E> Subject<'or, T, E> for PublishSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn terminated(&self) -> Option<Termination<E>> {
        // The resources are gone once the delivery stopped, which only an observer's panic does:
        // the subject is then dead, and reports no termination.
        self.0
            .update_resources(|resources| resources.termination.clone())
            .unwrap_or(None)
    }
}

/// One subscribed observer.
#[derive(Educe)]
#[educe(Debug)]
struct Entry<'or, T, E> {
    /// Identifies the entry before the delegate has added it, so a subscription can be disposed
    /// while its [`SubjectAction::Add`] is still queued.
    id: u64,
    /// Written by the disposal, read by the delegate before every notification.
    disposed: Shared<MutableBool>,
    observer: BoxedObserver<'or, T, E>,
}

/// Everything that reaches the observers, serialized by the delivery and applied outside its lock.
#[derive(Educe)]
#[educe(Debug)]
enum SubjectAction<'or, T, E> {
    /// Sends a value to every entry that is still subscribed.
    Forward(T),
    /// Adds an entry.
    Add(Entry<'or, T, E>),
    /// Removes the entry with this id, releasing its observer.
    Prune(u64),
    /// Terminates every entry. An observer that subscribes afterwards is terminated by
    /// [`Observable::subscribe`] instead.
    Terminate(Termination<E>),
}

/// Owns the observers, so that they are fed outside the lock that serializes the actions.
#[derive(Educe)]
#[educe(Debug)]
struct DelegateObserver<'or, T, E> {
    /// Sorted by id, which is handed out in subscription order: notifications follow that order,
    /// and an id is found by binary search.
    entries: Vec<Entry<'or, T, E>>,
}

impl<'or, T, E> Observer<SubjectAction<'or, T, E>, E> for DelegateObserver<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, action: SubjectAction<'or, T, E>) {
        match action {
            SubjectAction::Forward(value) => self.forward(value),
            SubjectAction::Add(entry) => self.add(entry),
            SubjectAction::Prune(id) => self.prune(id),
            SubjectAction::Terminate(termination) => self.terminate(termination),
        }
    }

    fn on_termination(self, _: Termination<E>) {
        debug_assert!(
            false,
            "the subject's delivery never terminates: see the module documentation"
        );
    }
}

impl<'or, T, E> DelegateObserver<'or, T, E>
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

    fn add(&mut self, entry: Entry<'or, T, E>) {
        if entry.disposed.read() {
            return; // Unsubscribed before it was ever added.
        }
        debug_assert!(
            self.entries.last().is_none_or(|last| last.id < entry.id),
            "the ids are handed out in subscription order"
        );
        self.entries.push(entry);
    }

    fn prune(&mut self, id: u64) {
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

struct PublishSubjectDisposal<'or, T, E> {
    delivery: SubjectDelivery<'or, T, E>,
    disposed: Shared<MutableBool>,
    id: u64,
}

impl<T, E> Disposable for PublishSubjectDisposal<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn dispose(self) {
        // The flag is what stops the events; the action only releases the observer afterwards.
        self.disposed.write(true);
        self.delivery
            .send(EventBatch::Next(SubjectAction::Prune(self.id)));
    }
}
