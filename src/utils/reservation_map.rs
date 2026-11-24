//! Keyed entries, each reserved before building the value it will hold.
//!
//! For an operator that holds any number of values built with the lock released, reserved one at
//! a time as they come: [`MergeAll`](crate::operators::combining::merge_all::MergeAll) and the
//! other operators that flatten an observable of observables hold the subscription of every inner
//! observable, [`Join`](crate::operators::combining::join::Join) the duration subscription of
//! every open window, one map per source. Each value is built in the same three steps:
//! **reserve** an entry under the lock, subscribe with it released, **fill** the entry under the
//! lock again. Between the two steps the entry can be **released** — the inner observable
//! completed, the window closed — and the fill then hands its value back instead of storing it.
//!
//! An entry is absent, reserved, finished or filled. Each entry is explicitly reserved under a key
//! that is never reused: a release removes it, and its late fill finds it absent. No separate
//! state for a release before the fill is needed. That is also what lets a late event of a
//! released entry, holding the stale key, reach nothing that came after it.
//!
//! A **finish** is the release of an entry whose place is not given up before its fill: the
//! inner observable completed while it was being subscribed, and its place stays counted until
//! the builder that reserved it returns. Its fill then removes it and hands the value back, so
//! that a place is only ever freed in one update, by whoever comes second, and can be handed on
//! in that same update. A release, by contrast, gives the place up at once: the window of a
//! [`Join`](crate::operators::combining::join::Join) closed, a
//! [`Switch`](crate::operators::combining::switch::Switch) replaced its inner observable.
//!
//! [`ReservationSlots`](crate::utils::positional::slots::ReservationSlots) runs the same protocol
//! over positions known in advance — the sources of a
//! [`Merge`](crate::operators::combining::merge::Merge) with a limit — and makes the opposite
//! choice: every position starts reserved, so a release before the fill must be recorded
//! ([`Released`](crate::utils::positional::slots::ReservationSlot::Released)),
//! where here an absent key already means released, since a key exists only from its reservation.
//! Those slots also hold a value of its own type at each position of a tuple, and drop what is
//! left the last position first; this map holds values of one type, and drops them in the order
//! they were reserved.
//!
//! The keys are unique within one map only: another map hands out the same ones, and so does a
//! fresh map put in place of this one. A key must therefore go back to the map that handed it
//! out, and a map with a fill still in flight must not be replaced by a new one; a map closed for
//! good is dropped instead, as [`Join`](crate::operators::combining::join::Join) does, so that the
//! late fill finds no map at all.
//!
//! Like those slots, the map carries no lock of its own — it lives inside a model guarded by a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) — and it
//! disposes nothing: each method hands the values it evicts back to the caller, which passes them
//! to
//! [`UpdateOutcome::with_drop_outside`](crate::utils::serialized_delivery::UpdateOutcome::with_drop_outside).

use crate::utils::id_generator::{Id, IdGenerator};
use educe::Educe;
use std::collections::BTreeMap;

/// A reservation's state, also returned when the entry is released.
#[derive(Educe)]
#[educe(Debug)]
pub(crate) enum Entry<D> {
    /// Its value is being built.
    Reserved,
    /// Its value is being built, and is no longer wanted: the fill removes the entry, which
    /// counts until then.
    Finished,
    /// It holds its value.
    Filled(D),
}

impl<D> Entry<D> {
    /// The value the entry held, if it was filled.
    #[must_use = "a released value must be dropped outside the lock that guards the map"]
    fn filled(self) -> Option<D> {
        match self {
            Self::Filled(value) => Some(value),
            Self::Reserved | Self::Finished => None,
        }
    }
}

/// Entries for values whose construction happens between reservation and filling, under keys
/// that are never reused.
///
/// `D` is the value being held — a
/// [`DisposeOnDrop`](crate::disposable::dispose_on_drop::DisposeOnDrop) in every current use, which
/// disposes when dropped.
#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ReservationMap<D> {
    /// Ordered by key, which is the order the entries were reserved in, so that dropping the map
    /// drops their values in that order.
    entries: BTreeMap<Id, Entry<D>>,
    keys: IdGenerator,
}

impl<D> ReservationMap<D> {
    /// Whether no entry is reserved, finished or filled.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries are reserved, finished or filled: the inner observables an operator is
    /// subscribed to, or subscribing to, which is what a limit on them counts. A finished entry
    /// counts until its fill: its place is not free before its builder has returned.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the entry `id` is reserved, finished or filled: it has not been released, nor
    /// removed by the fill of a finished entry.
    pub(crate) fn contains(&self, id: Id) -> bool {
        self.entries.contains_key(&id)
    }

    /// Reserves an entry for a value that is about to be built, and returns its key.
    #[must_use = "the key is the only way to fill or release the entry"]
    pub(crate) fn reserve(&mut self) -> Id {
        let id = self.keys.next_id();
        self.entries.insert(id, Entry::Reserved);
        id
    }

    /// Fills the reserved entry `id` with the value that was built.
    ///
    /// Returns `Some` when the entry was released or finished while the build was running, giving
    /// the value back to drop outside the lock. A finished entry is removed then: its place is
    /// free from this fill on.
    ///
    /// # Panics
    ///
    /// Panics if the entry is already filled: each reservation is filled once.
    #[must_use = "a value handed back must be dropped outside the lock that guards the map"]
    pub(crate) fn fill(&mut self, id: Id, value: D) -> Option<D> {
        match self.entries.get_mut(&id) {
            Some(Entry::Filled(_)) => panic!("the entry was filled twice"),
            Some(entry @ Entry::Reserved) => {
                *entry = Entry::Filled(value);
                None
            }
            Some(Entry::Finished) => {
                self.entries.remove(&id);
                Some(value)
            }
            None => Some(value),
        }
    }

    /// Removes the entry `id`, returning it to drop outside the lock, or `None` if absent.
    ///
    /// An entry returned unfilled, [`Reserved`](Entry::Reserved) or
    /// [`Finished`](Entry::Finished), still has its value being built: its [`fill`](Self::fill)
    /// hands the value back.
    #[must_use = "a released value must be dropped outside the lock that guards the map"]
    pub(crate) fn release(&mut self, id: Id) -> Option<Entry<D>> {
        self.entries.remove(&id)
    }

    /// Finishes the entry `id`: releases it if it is filled, and returns its value, to drop
    /// outside the lock; if its value is still being built, leaves it counted until its
    /// [`fill`](Self::fill), which removes it and hands the value back.
    #[must_use = "a released value must be dropped outside the lock that guards the map"]
    pub(crate) fn finish(&mut self, id: Id) -> Option<D> {
        match self.entries.get_mut(&id) {
            Some(entry @ Entry::Reserved) => {
                *entry = Entry::Finished;
                None
            }
            Some(Entry::Filled(_)) => self.release(id)?.filled(),
            Some(Entry::Finished) | None => None,
        }
    }

    /// Releases the entry reserved first among those left, and returns its value if it was
    /// filled, to drop outside the lock.
    ///
    /// This is the form used by a host that makes room for a new value by giving up the oldest
    /// one, as [`Switch`](crate::operators::combining::switch::Switch) does with the
    /// inner observable it replaces.
    #[must_use = "a released value must be dropped outside the lock that guards the map"]
    pub(crate) fn release_first(&mut self) -> Option<D> {
        let (_, entry) = self.entries.pop_first()?;
        entry.filled()
    }
}

#[cfg(test)]
mod tests;
