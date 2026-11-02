//! Keyed slots, each reserved before building the value it will hold.
//!
//! The keyed form of [`ReservationSlot`](crate::utils::reservation_slot::ReservationSlot), for an
//! operator that holds any number of values built with the lock released:
//! [`MergeAll`](crate::operators::combining::merge_all::MergeAll) holds the subscription of every
//! inner observable. Each value is built in the same three steps: **reserve** an entry under the
//! lock, subscribe with it released, **fill** the entry under the lock again. Between the two
//! steps the entry can be **released** — the inner observable completed — and the fill then hands
//! its value back instead of storing it.
//!
//! An entry is absent, reserved or filled. A slot needs a fourth state for a release that comes
//! before the fill, since the slot is reused afterwards; an entry is not: a release removes it,
//! and its key is never handed out again, so the late fill finds it absent. That is also what lets
//! a late event of a released entry, holding the stale key, reach nothing that came after it.
//!
//! The keys are unique within one map only: another map hands out the same ones, and so does a
//! fresh map put in place of this one. A key must therefore go back to the map that handed it
//! out, and a map with a fill still in flight must not be replaced by a new one; a map closed for
//! good is dropped instead, so that the late fill finds no map at all.
//!
//! Like the slot, the map carries no lock of its own — it lives inside a model guarded by a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) — and it
//! disposes nothing: each method hands the values it evicts back to the caller, which passes them
//! to
//! [`UpdateOutcome::with_drop_outside`](crate::utils::serialized_delivery::UpdateOutcome::with_drop_outside).

use crate::utils::id_generator::{Id, IdGenerator};
use educe::Educe;
use std::collections::BTreeMap;

/// Entries for values whose construction happens between reservation and filling, under keys
/// that are never reused.
///
/// `D` is the value being held — a
/// [`DisposeOnDrop`](crate::disposable::dispose_on_drop::DisposeOnDrop) in every current use, which
/// disposes when dropped.
#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ReservationMap<D> {
    /// `None` while the value of the entry is being built. Ordered by key, which is the order the
    /// entries were reserved in, so that dropping the map drops their values in that order.
    entries: BTreeMap<Id, Option<D>>,
    keys: IdGenerator,
}

/// What [`ReservationMap::release`] found under a key.
#[derive(Educe)]
#[educe(Debug)]
pub(crate) enum Released<D> {
    /// The entry was released already.
    Absent,
    /// The entry was reserved, and its value is still being built: its
    /// [`fill`](ReservationMap::fill) hands the value back.
    Reserved,
    /// The entry held this value, to drop outside the lock.
    Filled(D),
}

impl<D> ReservationMap<D> {
    /// Whether no entry is reserved or filled.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reserves an entry for a value that is about to be built, and returns its key.
    #[must_use = "the key is the only way to fill or release the entry"]
    pub(crate) fn reserve(&mut self) -> Id {
        let id = self.keys.next_id();
        self.entries.insert(id, None);
        id
    }

    /// Fills the reserved entry `id` with the value that was built.
    ///
    /// Returns `Some` when the entry was released while the build was running, giving the value
    /// back to drop outside the lock.
    ///
    /// # Panics
    ///
    /// Panics if the entry is already filled: each reservation is filled once.
    #[must_use = "a value handed back must be dropped outside the lock that guards the map"]
    pub(crate) fn fill(&mut self, id: Id, value: D) -> Option<D> {
        match self.entries.get_mut(&id) {
            Some(Some(_)) => panic!("the entry was filled twice"),
            Some(entry) => {
                *entry = Some(value);
                None
            }
            None => Some(value),
        }
    }

    /// Releases the entry `id`, and returns what it held, to drop outside the lock.
    #[must_use = "a released value must be dropped outside the lock that guards the map"]
    pub(crate) fn release(&mut self, id: Id) -> Released<D> {
        match self.entries.remove(&id) {
            None => Released::Absent,
            Some(None) => Released::Reserved,
            Some(Some(value)) => Released::Filled(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Released, ReservationMap};
    use std::{cell::Cell, rc::Rc};

    /// Counts the drops of the probes it hands out.
    struct DropCount(Rc<Cell<usize>>);

    struct Probe(Rc<Cell<usize>>);

    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    impl DropCount {
        fn new() -> Self {
            Self(Rc::new(Cell::new(0)))
        }

        fn get(&self) -> usize {
            self.0.get()
        }

        fn probe(&self) -> Probe {
            Probe(self.0.clone())
        }
    }

    #[test]
    fn test_reserve_fill_release() {
        let drops = DropCount::new();
        let mut map = ReservationMap::default();
        let id = map.reserve(); // Step 1, under the lock.
        assert!(map.fill(id, drops.probe()).is_none()); // Step 3, under the lock: stored.
        assert_eq!(drops.get(), 0);

        let released = map.release(id);
        assert!(matches!(released, Released::Filled(_)));
        assert!(map.is_empty());
        assert_eq!(drops.get(), 0); // Handed back, to be dropped outside the lock.
        drop(released);
        assert_eq!(drops.get(), 1);

        assert!(matches!(map.release(id), Released::Absent)); // Released once only.
    }

    #[test]
    fn test_release_before_fill_hands_value_back() {
        let drops = DropCount::new();
        let mut map = ReservationMap::default();
        let id = map.reserve();
        assert!(matches!(map.release(id), Released::Reserved));
        assert!(map.is_empty());

        let late = map.fill(id, drops.probe());
        assert!(late.is_some());
        assert!(map.is_empty());
        assert_eq!(drops.get(), 0);
        drop(late);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn test_keys_are_not_reused() {
        let drops = DropCount::new();
        let mut map = ReservationMap::default();
        let first = map.reserve();
        assert!(matches!(map.release(first), Released::Reserved));
        let second = map.reserve();
        assert_ne!(first, second);

        // The late fill of the first entry does not reach the second.
        let late = map.fill(first, drops.probe());
        assert!(late.is_some());
        assert!(map.fill(second, drops.probe()).is_none());
        drop(late);
        assert_eq!(drops.get(), 1);

        let released = map.release(second);
        assert!(matches!(released, Released::Filled(_)));
        assert_eq!(drops.get(), 1);
        drop(released);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    #[should_panic(expected = "the entry was filled twice")]
    fn test_fill_rejects_filled_entry() {
        let mut map = ReservationMap::default();
        let id = map.reserve();
        assert!(map.fill(id, ()).is_none());
        let _ = map.fill(id, ());
    }
}
