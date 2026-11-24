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
