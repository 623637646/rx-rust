//! One-shot slots addressed by position, independent of the values they hold.
//!
//! A position is reserved before building its value outside the lock that owns the store. A
//! release can arrive before the fill; the fill then hands the value back. Every operation
//! returns any value it removes, so the caller can drop it outside the lock. Tuple stores retain
//! a slot per position; list stores remove entries once both operations have finished.

use super::{
    position::AtPosition,
    tuple::{IntoReversed, for_each_arity},
};
use educe::Educe;
use std::collections::{BTreeMap, btree_map::Entry};

/// A one-shot slot, initially reserved, that is filled and released in either order. A release
/// is final: whatever is filled afterwards is handed back.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug)]
pub enum ReservationSlot<V> {
    /// The position is reserved for a value that has not been filled yet.
    Reserved,
    /// A value is held until the slot is released.
    Filled(V),
    /// The slot was released, before its fill or after it: a fill hands its value back.
    Released,
}

impl<V> ReservationSlot<V> {
    /// Fills the slot, or hands the value back if the slot was released before the fill.
    ///
    /// # Panics
    ///
    /// Panics if the slot holds a value.
    #[must_use = "a value handed back must be dropped outside the lock that guards the slot"]
    pub(crate) fn fill(&mut self, value: V) -> Option<V> {
        match self {
            Self::Reserved => {
                *self = Self::Filled(value);
                None
            }
            Self::Released => Some(value),
            Self::Filled(_) => panic!("the slot was already filled"),
        }
    }

    /// Releases the slot, returning its value if one is stored. A release before the fill
    /// makes the later fill hand its value back.
    ///
    /// # Panics
    ///
    /// Panics if the slot was already released.
    #[must_use = "a released value must be dropped outside the lock that guards the slot"]
    pub(crate) fn release(&mut self) -> Option<V> {
        match std::mem::replace(self, Self::Released) {
            Self::Reserved => None,
            Self::Filled(value) => Some(value),
            Self::Released => panic!("the slot was already released"),
        }
    }
}

/// A collection of slots, each implicitly reserved, filled once and released once.
///
/// Filling and releasing may happen in either order. A release before the fill records that the
/// value is no longer wanted, so the later fill hands it back instead of storing it. Each method
/// returns any value it removes; the caller decides when to drop it, outside any lock guarding
/// the collection.
pub(crate) trait ReservationSlots<K> {
    /// The value held at `K`.
    type Value;

    /// Fills the slot at `position`, or hands the value back if the slot was already released.
    #[must_use = "a value handed back must be dropped outside the lock that guards the slots"]
    fn fill(&mut self, position: K, value: Self::Value) -> Option<Self::Value>;

    /// Releases the slot at `position`, returning its value if one is stored.
    #[must_use = "a released value must be dropped outside the lock that guards the slots"]
    fn release(&mut self, position: K) -> Option<Self::Value>;
}

/// One reservation per tuple position, initially reserved, retaining each value's type.
/// Dropped, the remaining values are dropped in reverse position order.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug)]
pub struct TupleSlots<T: IntoReversed>(Option<T>);

impl<T: IntoReversed> Drop for TupleSlots<T> {
    fn drop(&mut self) {
        if let Some(slots) = self.0.take() {
            drop(slots.into_reversed());
        }
    }
}

impl<T, K, V> ReservationSlots<K> for TupleSlots<T>
where
    T: IntoReversed + AtPosition<K, Element = ReservationSlot<V>>,
{
    type Value = V;

    fn fill(&mut self, position: K, value: V) -> Option<V> {
        self.slots_mut().at_mut(position).fill(value)
    }

    fn release(&mut self, position: K) -> Option<V> {
        self.slots_mut().at_mut(position).release()
    }
}

impl<T: IntoReversed> TupleSlots<T> {
    fn slots_mut(&mut self) -> &mut T {
        self.0
            .as_mut()
            .expect("the slots are taken out only as they are dropped")
    }
}

macro_rules! impl_tuple_slots {
    ($(($_position:tt, $value:ident, $($_names:tt)*)),+) => {
        impl<$($value),+> Default for TupleSlots<($(ReservationSlot<$value>,)+)> {
            fn default() -> Self {
                Self(Some(($(ReservationSlot::<$value>::Reserved,)+)))
            }
        }
    };
}

for_each_arity!(impl_tuple_slots);

/// Sparse reservations addressed by position, retaining only values or releases awaiting a fill.
/// An absent position needs no storage until its first fill or release. Dropped, the remaining
/// values are dropped in reverse position order.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug, Default(bound()))]
pub struct ListSlots<V>(BTreeMap<usize, Option<V>>);

impl<V> Drop for ListSlots<V> {
    fn drop(&mut self) {
        // A vector drops front to back and still drops the rest if one value's drop panics.
        let values: Vec<_> = std::mem::take(&mut self.0).into_values().rev().collect();
        drop(values);
    }
}

impl<V> ReservationSlots<usize> for ListSlots<V> {
    type Value = V;

    fn fill(&mut self, position: usize, value: V) -> Option<V> {
        match self.0.entry(position) {
            Entry::Vacant(entry) => {
                entry.insert(Some(value));
                None
            }
            Entry::Occupied(entry) => {
                assert!(entry.get().is_none(), "the value was already stored");
                entry.remove();
                Some(value)
            }
        }
    }

    fn release(&mut self, position: usize) -> Option<V> {
        match self.0.entry(position) {
            Entry::Vacant(entry) => {
                entry.insert(None);
                None
            }
            Entry::Occupied(entry) => {
                assert!(entry.get().is_some(), "the slot was already released");
                entry.remove()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ListSlots, ReservationSlot, ReservationSlots, TupleSlots};
    use crate::{thread_mode::mutable::MutableHelper, utils::positional::position::TupleIndex};
    use std::{cell::RefCell, rc::Rc};

    struct Probe<'a>(&'a RefCell<Vec<usize>>, usize);

    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            self.0.with_mut(|order| order.push(self.1));
        }
    }

    #[test]
    fn test_fill_then_release() {
        let order = RefCell::new(Vec::new());
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.fill(Probe(&order, 0)).is_none());
        assert!(matches!(slot, ReservationSlot::Filled(_)));

        let finished = slot.release();
        assert!(finished.is_some());
        assert!(matches!(slot, ReservationSlot::Released));
        order.with_ref(|order| assert!(order.is_empty()));
        drop(finished);
        order.with_ref(|order| assert_eq!(*order, [0]));
    }

    #[test]
    fn test_release_before_fill_returns_late_value() {
        let order = RefCell::new(Vec::new());
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.release().is_none());
        assert!(matches!(slot, ReservationSlot::Released));

        let late = slot.fill(Probe(&order, 0));
        assert!(late.is_some());
        assert!(matches!(slot, ReservationSlot::Released));
        order.with_ref(|order| assert!(order.is_empty()));
        drop(late);
        order.with_ref(|order| assert_eq!(*order, [0]));
    }

    #[test]
    fn test_fill_after_release_returns_value() {
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.fill(0).is_none());
        assert_eq!(slot.release(), Some(0));
        // The release is final: nothing is stored again.
        assert_eq!(slot.fill(1), Some(1));
        assert!(matches!(slot, ReservationSlot::Released));
    }

    #[test]
    #[should_panic(expected = "the slot was already filled")]
    fn test_fill_rejects_filled_slot() {
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.fill(()).is_none());
        let _ = slot.fill(());
    }

    #[test]
    #[should_panic(expected = "the slot was already released")]
    fn test_release_rejects_released_slot_without_value() {
        let _ = ReservationSlot::<()>::Released.release();
    }

    #[test]
    fn test_release_rejects_released_slot() {
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.release().is_none());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| slot.release()));
        assert!(result.is_err());
        // A rejected duplicate release must still hand the pending value back when it arrives.
        assert!(matches!(slot, ReservationSlot::Released));
        assert_eq!(slot.fill(()), Some(()));
    }

    #[test]
    #[should_panic(expected = "the slot was already released")]
    fn test_release_rejects_already_released_value() {
        let mut slot = ReservationSlot::Reserved;
        assert!(slot.fill(()).is_none());
        assert_eq!(slot.release(), Some(()));
        let _ = slot.release();
    }

    #[test]
    fn test_tuple_slots() {
        let order = RefCell::new(Vec::new());
        let value = |position| Probe(&order, position);
        let mut slots = TupleSlots::<(_, _, _, _)>::default();
        assert!(slots.fill(TupleIndex::<0>, value(0)).is_none());
        assert!(slots.fill(TupleIndex::<1>, value(1)).is_none());
        assert!(slots.fill(TupleIndex::<2>, value(2)).is_none());

        let released = slots.release(TupleIndex::<1>);
        order.with_ref(|order| assert!(order.is_empty()));
        drop(released);
        order.with_ref(|order| assert_eq!(*order, [1]));

        assert!(slots.release(TupleIndex::<3>).is_none());
        drop(slots.fill(TupleIndex::<3>, value(3)));
        order.with_ref(|order| assert_eq!(*order, [1, 3]));

        drop(slots);
        order.with_ref(|order| assert_eq!(*order, [1, 3, 2, 0]));
    }

    #[test]
    fn test_tuple_slots_with_heterogeneous_values() {
        let mut value = 0;
        let mut slots = TupleSlots::<(_, _, _)>::default();
        assert!(slots.fill(TupleIndex::<0>, String::from("a")).is_none());
        assert!(slots.fill(TupleIndex::<1>, &mut value).is_none());
        assert!(slots.fill(TupleIndex::<2>, Rc::new(2)).is_none());

        assert_eq!(slots.release(TupleIndex::<0>).as_deref(), Some("a"));
        *slots.release(TupleIndex::<1>).unwrap() = 3;
        assert_eq!(slots.release(TupleIndex::<2>).as_deref(), Some(&2));
        drop(slots);
        assert_eq!(value, 3);
    }

    #[test]
    fn test_list_slots() {
        let order = RefCell::new(Vec::new());
        let value = |position| Probe(&order, position);
        let mut slots = ListSlots::default();
        for position in 0..4 {
            assert!(slots.fill(position, value(position)).is_none());
        }

        let released = slots.release(1);
        assert_eq!(slots.0.len(), 3);
        order.with_ref(|order| assert!(order.is_empty()));
        drop(released);
        order.with_ref(|order| assert_eq!(*order, [1]));

        assert!(slots.release(4).is_none());
        assert_eq!(slots.0.len(), 4);
        let late = slots.fill(4, value(4));
        assert_eq!(slots.0.len(), 3);
        drop(late);
        order.with_ref(|order| assert_eq!(*order, [1, 4]));

        drop(slots);
        order.with_ref(|order| assert_eq!(*order, [1, 4, 3, 2, 0]));
    }

    #[cfg(panic = "unwind")]
    struct DropPanic;

    #[cfg(panic = "unwind")]
    struct PanicProbe<'a> {
        order: &'a RefCell<Vec<usize>>,
        position: usize,
        panic_position: usize,
    }

    #[cfg(panic = "unwind")]
    impl Drop for PanicProbe<'_> {
        fn drop(&mut self) {
            self.order.with_mut(|order| order.push(self.position));
            if self.position == self.panic_position {
                std::panic::resume_unwind(Box::new(DropPanic));
            }
        }
    }

    #[cfg(panic = "unwind")]
    fn check_drop_remaining_on_panic(drop_slots: impl Fn(&RefCell<Vec<usize>>, usize)) {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        for panic_position in 0..4 {
            let order = RefCell::new(Vec::new());
            let result = catch_unwind(AssertUnwindSafe(|| drop_slots(&order, panic_position)));
            let payload = result.expect_err("the selected value's drop must panic");
            assert!(payload.is::<DropPanic>());
            order.with_ref(|order| assert_eq!(*order, [3, 2, 1, 0]));
        }
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_tuple_slots_drop_remaining_on_panic() {
        check_drop_remaining_on_panic(|order, panic_position| {
            let value = |position| PanicProbe {
                order,
                position,
                panic_position,
            };
            let mut slots = TupleSlots::<(_, _, _, _)>::default();
            assert!(slots.fill(TupleIndex::<0>, value(0)).is_none());
            assert!(slots.fill(TupleIndex::<1>, value(1)).is_none());
            assert!(slots.fill(TupleIndex::<2>, value(2)).is_none());
            assert!(slots.fill(TupleIndex::<3>, value(3)).is_none());
            drop(slots);
        });
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_list_slots_drop_remaining_on_panic() {
        check_drop_remaining_on_panic(|order, panic_position| {
            let mut slots = ListSlots::default();
            for position in 0..4 {
                let value = PanicProbe {
                    order,
                    position,
                    panic_position,
                };
                assert!(slots.fill(position, value).is_none());
            }
            drop(slots);
        });
    }
}
