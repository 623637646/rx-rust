//! A slot reserved before building the value it will hold, such as a subscription or task handle.
//!
//! An operator such as [`Switch`](crate::operators::combining::switch::Switch) or
//! [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) is subscribed to at most one
//! inner observable at a time, and swaps that subscription as the source emits. Subscribing is an
//! external API call, so it must happen with no lock held, which splits every swap into two
//! locked steps around an unlocked one: **reserve** the slot, subscribe, **fill** the slot.
//! Between the two steps the inner observable can complete and release the slot. That release
//! keeps the reservation occupied until the fill returns the new subscription instead of storing
//! it: another build must not reuse the slot while the first one is still returning.
//! The same reserve/build/fill sequence holds a scheduled task in
//! [`ObserveOn`](crate::operators::utility::observe_on::ObserveOn) or a timer in
//! [`Delay`](crate::operators::utility::delay::Delay).
//!
//! [`ReservationSlot`] is that three-step state machine and nothing else. It carries no lock of
//! its own — it lives inside a model guarded by a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) — and it
//! disposes nothing: each method hands the value it evicts back to the caller, which passes it to
//! [`UpdateOutcome::with_drop_outside`](crate::utils::serialized_delivery::UpdateOutcome::with_drop_outside).
//!
//! [`Reserved`](ReservationSlot::Reserved) and
//! [`ReleasedWhileReserved`](ReservationSlot::ReleasedWhileReserved) distinguish a build in
//! flight from an idle slot. Use this for a *single* value built with the lock released;
//! several of them are keyed, and an absent key already means idle, as in
//! [`MergeAll`](crate::operators::combining::merge_all::MergeAll). It is not built on
//! [`ReplaceableDisposal`](crate::disposable::replaceable_disposal::ReplaceableDisposal), which is
//! itself a disposal with its own lock and a terminal state — neither of which a slot needs.
//!
//! # Which reserve to use
//!
//! [`reserve_replacing`](ReservationSlot::reserve_replacing) panics when a build is in flight,
//! so it may only be used by a host with **one reserving caller, serialized against itself**.
//! [`Switch`](crate::operators::combining::switch::Switch) qualifies: only its source observer
//! reserves, and `Observer::on_next` takes `&mut self`, so the source's own delivery serializes
//! every call — a value the inner emits back into the source while its subscription is being
//! built is queued by that delivery, not delivered re-entrantly. Its inner observer only ever
//! releases.
//!
//! A host with a **second** reserving caller has no such guarantee and must use
//! [`reserve_if_idle`](ReservationSlot::reserve_if_idle) plus a queue.
//! [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) is the example: its source
//! observer reserves for the next observable, and so does the continuation that runs when an
//! inner completes. Those two belong to different deliveries and are not serialized against each
//! other, so either may find a build in flight and must queue instead of reserving. Answering
//! `false` is exactly what keeps the queue in order.

use educe::Educe;

/// A slot for one value whose construction happens between reservation and filling.
///
/// `D` is the value being held — a
/// [`DisposeOnDrop`](crate::disposable::dispose_on_drop::DisposeOnDrop) in every current use, which
/// disposes when dropped.
#[derive(Educe)]
#[educe(Debug)]
pub enum ReservationSlot<D> {
    /// No value is held and no build is in flight.
    Idle,
    /// A value is being built with the lock released. The reservation tells a concurrent update
    /// that a build is in flight.
    Reserved,
    /// The slot was released before its build returned. The reservation stays occupied until
    /// `fill` hands back the built value and makes the slot idle.
    ReleasedWhileReserved,
    /// A value is held.
    Active(D),
}

impl<D> ReservationSlot<D> {
    /// Whether the slot is [`Idle`](Self::Idle).
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Reserves the slot for a value that is about to be built, and gives back the value it
    /// replaces, if any, to drop outside the lock.
    ///
    /// # Panics
    ///
    /// Panics if a build is still in flight, including after a release: only one build may be
    /// outstanding, and it must fill the slot before another can reserve it.
    pub fn reserve_replacing(&mut self) -> Option<D> {
        match self {
            // Reject a second build before touching the state, so that a caught panic leaves the
            // first one's reservation intact.
            Self::Reserved | Self::ReleasedWhileReserved => {
                unreachable!("the slot is already reserved")
            }
            Self::Idle => {
                *self = Self::Reserved;
                None
            }
            Self::Active(_) => match std::mem::replace(self, Self::Reserved) {
                Self::Active(value) => Some(value),
                _ => unreachable!(),
            },
        }
    }

    /// Reserves the slot only if it is [`Idle`](Self::Idle), so nothing is ever evicted, and
    /// returns whether it did.
    ///
    /// This is the form used by a host that keeps one task alive while there is work to do: it
    /// starts a task only when none is running, instead of replacing a running one.
    #[must_use = "a reservation that is not followed by a build leaves the slot reserved forever"]
    pub fn reserve_if_idle(&mut self) -> bool {
        match self {
            Self::Idle => {
                *self = Self::Reserved;
                true
            }
            Self::Reserved | Self::ReleasedWhileReserved | Self::Active(_) => false,
        }
    }

    /// Fills a reserved slot with the value that was built.
    ///
    /// Returns `Some` when the slot was released while the build was running, in which case the
    /// slot becomes idle and the value is given back to drop outside the lock. If this starts
    /// another build, decide what to build and reserve it under the same lock as this fill.
    ///
    /// # Panics
    ///
    /// Panics if no build is in flight: the slot is idle or already active.
    pub fn fill(&mut self, value: D) -> Option<D> {
        match self {
            Self::Reserved => {
                *self = Self::Active(value);
                None
            }
            Self::ReleasedWhileReserved => {
                *self = Self::Idle;
                Some(value)
            }
            Self::Idle | Self::Active(_) => {
                unreachable!("the slot was filled without being reserved")
            }
        }
    }

    /// Releases the slot, giving back the held value, if any, to drop outside the lock.
    ///
    /// Releasing a reserved slot returns `None` and keeps the reservation occupied until the
    /// pending [`fill`](Self::fill) gives its value back. Releasing an active slot makes
    /// it idle and returns its value; if this starts another build, reserve it
    /// under the same lock as this release.
    ///
    /// # Panics
    ///
    /// Panics if the slot is idle or was already released while its build is still in flight.
    /// Each reservation may be released only once.
    pub fn release(&mut self) -> Option<D> {
        match self {
            Self::Reserved => {
                *self = Self::ReleasedWhileReserved;
                None
            }
            Self::Active(_) => match std::mem::replace(self, Self::Idle) {
                Self::Active(value) => Some(value),
                _ => unreachable!(),
            },
            Self::Idle => unreachable!("the slot was released without being reserved"),
            Self::ReleasedWhileReserved => unreachable!("the slot was already released"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReservationSlot;
    use std::{cell::Cell, rc::Rc};

    /// Counts the drops of the probes it hands out.
    struct DropCount(Rc<Cell<usize>>);

    struct Probe(Rc<Cell<usize>>);

    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// Whether a build is still in flight, even if its value has already been released.
    fn is_reserved<D>(slot: &ReservationSlot<D>) -> bool {
        matches!(
            slot,
            ReservationSlot::Reserved | ReservationSlot::ReleasedWhileReserved
        )
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
    fn test_reserve_fill_evict() {
        let drops = DropCount::new();
        let mut slot = ReservationSlot::Idle;
        assert!(slot.reserve_if_idle()); // Step 1, under the lock.
        let value = drops.probe(); // Step 2, built with the lock released.
        assert!(slot.fill(value).is_none()); // Step 3, under the lock: stored.

        let evicted = slot.reserve_replacing(); // The next swap evicts the stored value…
        assert!(evicted.is_some()); // …to be disposed outside the lock.
        assert_eq!(drops.get(), 0);
        drop(evicted);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn test_release_before_fill_keeps_reservation() {
        let drops = DropCount::new();
        let mut slot = ReservationSlot::Idle;
        assert!(slot.reserve_if_idle());

        assert!(slot.release().is_none());
        assert!(!slot.is_idle());
        assert!(is_reserved(&slot));
        assert!(!slot.reserve_if_idle());

        let finished = slot.fill(drops.probe());
        assert!(finished.is_some());
        assert!(slot.is_idle());
        assert_eq!(drops.get(), 0);

        // A later build is independent of the finished value returned to the caller.
        assert!(slot.reserve_if_idle());
        assert!(slot.fill(drops.probe()).is_none());
        drop(finished);
        assert_eq!(drops.get(), 1);
        assert!(!slot.is_idle());
        assert!(!is_reserved(&slot));

        let finished = slot.release();
        assert!(finished.is_some());
        assert!(slot.is_idle());
        assert_eq!(drops.get(), 1);
        drop(finished);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn test_fill_before_release_hands_back_value() {
        let drops = DropCount::new();
        let mut slot = ReservationSlot::Idle;
        assert!(slot.reserve_if_idle());
        assert!(slot.fill(drops.probe()).is_none());
        assert!(!slot.reserve_if_idle());

        let finished = slot.release();
        assert!(finished.is_some());
        assert!(slot.is_idle());
        assert_eq!(drops.get(), 0);
        drop(finished);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn test_reserve_replacing_active_value() {
        let drops = DropCount::new();
        let mut slot = ReservationSlot::Idle;
        assert!(slot.reserve_replacing().is_none());
        assert!(slot.fill(drops.probe()).is_none());

        let evicted = slot.reserve_replacing();
        assert!(evicted.is_some());
        assert!(is_reserved(&slot));
        assert_eq!(drops.get(), 0);
        drop(evicted);
        assert_eq!(drops.get(), 1);

        assert!(slot.release().is_none());
        let finished = slot.fill(drops.probe());
        assert!(finished.is_some());
        assert!(slot.is_idle());
        drop(finished);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    #[should_panic(expected = "the slot is already reserved")]
    fn test_reserve_replacing_rejects_released_build_in_flight() {
        let mut slot = ReservationSlot::<()>::Idle;
        assert!(slot.reserve_if_idle());
        assert!(slot.release().is_none());
        slot.reserve_replacing();
    }

    #[test]
    #[should_panic(expected = "the slot was released without being reserved")]
    fn test_release_rejects_idle_slot() {
        ReservationSlot::<()>::Idle.release();
    }

    #[test]
    fn test_release_rejects_released_build_in_flight() {
        let mut slot = ReservationSlot::<()>::Idle;
        assert!(slot.reserve_if_idle());
        assert!(slot.release().is_none());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| slot.release()));
        assert!(result.is_err());
        // Reject the duplicate before changing state: catching the panic must not free the slot.
        assert!(matches!(slot, ReservationSlot::ReleasedWhileReserved));
        assert!(!slot.reserve_if_idle());
        assert_eq!(slot.fill(()), Some(()));
        assert!(slot.is_idle());
    }

    #[test]
    #[should_panic(expected = "the slot was released without being reserved")]
    fn test_release_rejects_already_released_value() {
        let mut slot = ReservationSlot::<()>::Idle;
        assert!(slot.reserve_if_idle());
        assert!(slot.fill(()).is_none());
        assert_eq!(slot.release(), Some(()));
        slot.release();
    }
}
