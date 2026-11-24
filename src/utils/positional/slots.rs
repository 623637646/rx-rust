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
mod tests;
