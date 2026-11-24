use super::{Entry, ReservationMap};
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
    assert!(map.contains(id));
    assert!(map.fill(id, drops.probe()).is_none()); // Step 3, under the lock: stored.
    assert_eq!(drops.get(), 0);

    let released = map.release(id);
    assert!(matches!(released, Some(Entry::Filled(_))));
    assert!(!map.contains(id));
    assert!(map.is_empty());
    assert_eq!(drops.get(), 0); // Handed back, to be dropped outside the lock.
    drop(released);
    assert_eq!(drops.get(), 1);

    assert!(map.release(id).is_none()); // Released once only.
}

#[test]
fn test_release_before_fill_hands_value_back() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    let id = map.reserve();
    assert!(matches!(map.release(id), Some(Entry::Reserved)));
    assert!(map.is_empty());

    let late = map.fill(id, drops.probe());
    assert!(late.is_some());
    assert!(map.is_empty());
    assert_eq!(drops.get(), 0);
    drop(late);
    assert_eq!(drops.get(), 1);
}

#[test]
fn test_len_counts_reserved_and_filled_entries() {
    let mut map = ReservationMap::default();
    assert_eq!(map.len(), 0);
    let first = map.reserve();
    let second = map.reserve();
    assert_eq!(map.len(), 2);
    assert!(map.fill(first, ()).is_none());
    assert_eq!(map.len(), 2);
    assert!(matches!(map.release(second), Some(Entry::Reserved)));
    assert_eq!(map.len(), 1);
    assert!(matches!(map.release(first), Some(Entry::Filled(()))));
    assert_eq!(map.len(), 0);
}

#[test]
fn test_keys_are_not_reused() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    let first = map.reserve();
    assert!(matches!(map.release(first), Some(Entry::Reserved)));
    let second = map.reserve();
    assert_ne!(first, second);

    // The late fill of the first entry does not reach the second.
    let late = map.fill(first, drops.probe());
    assert!(late.is_some());
    assert!(map.fill(second, drops.probe()).is_none());
    drop(late);
    assert_eq!(drops.get(), 1);

    let released = map.release(second);
    assert!(matches!(released, Some(Entry::Filled(_))));
    assert_eq!(drops.get(), 1);
    drop(released);
    assert_eq!(drops.get(), 2);
}

#[test]
fn test_release_first_releases_in_reservation_order() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    assert!(map.release_first().is_none());

    let first = map.reserve();
    let second = map.reserve();
    assert!(map.fill(second, drops.probe()).is_none());

    // The first entry is still being built: its late fill hands the value back.
    assert!(map.release_first().is_none());
    assert!(!map.contains(first));
    assert!(map.contains(second));
    let late = map.fill(first, drops.probe());
    assert!(late.is_some());
    drop(late);
    assert_eq!(drops.get(), 1);

    let released = map.release_first();
    assert!(released.is_some());
    assert!(map.is_empty());
    assert_eq!(drops.get(), 1); // Handed back, to be dropped outside the lock.
    drop(released);
    assert_eq!(drops.get(), 2);
}

#[test]
fn test_finish_before_fill_keeps_the_entry_counted_until_the_fill() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    let id = map.reserve();
    assert!(map.finish(id).is_none());
    assert!(map.contains(id));
    assert_eq!(map.len(), 1); // The place is not free before the builder returns.
    assert!(map.finish(id).is_none()); // Finishing twice changes nothing.
    assert!(map.contains(id));

    let late = map.fill(id, drops.probe());
    assert!(late.is_some());
    assert!(map.is_empty()); // Free from the fill on.
    drop(late);
    assert_eq!(drops.get(), 1);
    assert!(map.finish(id).is_none());
}

#[test]
fn test_finish_after_fill_releases() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    let id = map.reserve();
    assert!(map.fill(id, drops.probe()).is_none());
    let released = map.finish(id);
    assert!(released.is_some());
    assert!(map.is_empty());
    drop(released);
    assert_eq!(drops.get(), 1);
}

#[test]
fn test_release_of_a_finished_entry_gives_its_place_up_at_once() {
    let drops = DropCount::new();
    let mut map = ReservationMap::default();
    let id = map.reserve();
    assert!(map.finish(id).is_none());
    assert!(map.release_first().is_none());
    assert!(map.is_empty());
    let late = map.fill(id, drops.probe());
    assert!(late.is_some());
    drop(late);
    assert_eq!(drops.get(), 1);
}

#[test]
#[should_panic(expected = "the entry was filled twice")]
fn test_fill_rejects_filled_entry() {
    let mut map = ReservationMap::default();
    let id = map.reserve();
    assert!(map.fill(id, ()).is_none());
    let _ = map.fill(id, ());
}
