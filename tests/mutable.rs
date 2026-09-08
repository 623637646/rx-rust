//! The guarantees of the [`Mutable`] API: what runs under the lock, and what does not.

use rx_rust::utils::mutable::{Mutable, MutableExt, MutableHelper};
use rx_rust::utils::types::Shared;

/// A value whose drop takes the lock it was stored in, which deadlocks if it is dropped under it.
struct DropsIntoItsOwnLock(Shared<Mutable<Vec<DropsIntoItsOwnLock>>>);

impl Drop for DropsIntoItsOwnLock {
    fn drop(&mut self) {
        self.0
            .with_mut(|values| values.push(DropsIntoItsOwnLock(self.0.clone())));
    }
}

#[test]
fn test_take_value_drops_outside_the_lock() {
    let values: Shared<Mutable<Vec<DropsIntoItsOwnLock>>> = Shared::new(Mutable::new(Vec::new()));
    values.with_mut(|slot| slot.push(DropsIntoItsOwnLock(values.clone())));

    // `with_mut(Vec::clear)` would drop the element under the lock, and its drop takes that lock
    // again. Taking the vec out hands the drop to this scope, where the lock is free.
    drop(values.take_value());

    // The drop above re-entered the lock and left its own replacement behind, which proves it ran
    // outside. That replacement is dropped by the same route.
    assert_eq!(values.with_ref(Vec::len), 1);
    drop(values.take_value());
}

#[test]
fn test_replace_value_drops_outside_the_lock() {
    let values: Shared<Mutable<Vec<DropsIntoItsOwnLock>>> = Shared::new(Mutable::new(Vec::new()));
    values.with_mut(|slot| slot.push(DropsIntoItsOwnLock(values.clone())));

    drop(values.replace_value(Vec::new()));

    assert_eq!(values.with_ref(Vec::len), 1);
    drop(values.take_value());
}

#[test]
fn test_take_value_hands_an_option_over_outside_the_lock() {
    let slot: Mutable<Option<i32>> = Mutable::new(Some(1));

    let taken = slot.take_value().map(|value| {
        // The lock is already released, so the mapping may take it again.
        slot.with_mut(|slot| *slot = Some(value + 1));
        value * 10
    });

    assert_eq!(taken, Some(10));
    assert_eq!(slot.clone_value(), Some(2));
}

#[test]
fn test_take_value_on_an_empty_option() {
    let slot: Mutable<Option<i32>> = Mutable::new(None);
    assert_eq!(slot.take_value(), None);
}

/// Re-entering the same lock deadlocks a `Mutex` and panics a `RefCell`, so debug builds report it
/// at the call site instead.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "already held by the current thread")]
fn test_reentrant_lock_panics() {
    let value = Mutable::new(0);
    value.with_mut(|_| value.with_mut(|value| *value += 1));
}

/// Two different locks nest freely: only the same one twice is a deadlock.
#[test]
fn test_nested_locks_are_allowed() {
    let first = Mutable::new(1);
    let second = Mutable::new(2);
    let sum = first.with_ref(|first| second.with_ref(|second| first + second));
    assert_eq!(sum, 3);
}
