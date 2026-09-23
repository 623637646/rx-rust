//! The slot that holds the one inner subscription an operator keeps at a time.
//!
//! An operator such as [`Switch`](crate::operators::combining::switch::Switch) or
//! [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) is subscribed to at most one
//! inner observable at a time, and swaps that subscription as the source emits. Subscribing is an
//! external API call, so it must happen with no lock held, which splits every swap into two
//! locked steps around an unlocked one: **reserve** the slot, subscribe, **fill** the slot.
//! Between the two steps the inner observable can complete and release the slot. That release
//! keeps the reservation occupied until the fill returns the new subscription instead of storing
//! it: another build must not reuse the slot while the first one is still returning.
//!
//! [`SubscriptionSlot`] is that three-step state machine and nothing else. It carries no lock of
//! its own — it lives inside a model guarded by a
//! [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) — and it
//! disposes nothing: each method hands the subscription it evicts back to the caller, which passes
//! it to [`UpdateOutcome::with_drop_outside`](crate::utils::serialized_delivery::UpdateOutcome::with_drop_outside).
//!
//! [`Reserved`](SubscriptionSlot::Reserved) and
//! [`ReleasedWhileReserved`](SubscriptionSlot::ReleasedWhileReserved) distinguish a build in
//! flight from an idle slot. Use this for a *single* subscription built with the lock released;
//! several of them are keyed, and an absent key already means idle, as in
//! [`MergeAll`](crate::operators::combining::merge_all::MergeAll). It is not built on
//! [`SharedDisposal`](crate::disposable::shared_disposal::SharedDisposal), which is itself a
//! disposal with its own lock and a terminal state — neither of which a slot needs.
//!
//! # Which reserve to use
//!
//! [`reserve_replacing`](SubscriptionSlot::reserve_replacing) panics when a build is in flight,
//! so it may only be used by a host with **one reserving caller, serialized against itself**.
//! [`Switch`](crate::operators::combining::switch::Switch) qualifies: only its source observer
//! reserves, and `Observer::on_next` takes `&mut self`, so the source's own delivery serializes
//! every call — a value the inner emits back into the source while its subscription is being
//! built is queued by that delivery, not delivered re-entrantly. Its inner observer only ever
//! releases.
//!
//! A host with a **second** reserving caller has no such guarantee and must use
//! [`reserve_if_idle`](SubscriptionSlot::reserve_if_idle) plus a queue.
//! [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) is the example: its source
//! observer reserves for the next observable, and so does the continuation that runs when an
//! inner completes. Those two belong to different deliveries and are not serialized against each
//! other, so either may find a build in flight and must queue instead of reserving. Answering
//! `false` is exactly what keeps the queue in order.
//!
//! # Examples
//! ```rust
//! use rx_rust::{disposable::callback_disposal::CallbackDisposal, utils::subscription_slot::SubscriptionSlot};
//!
//! let mut slot = SubscriptionSlot::Idle;
//! assert!(slot.reserve_if_idle()); // Step 1, under the lock.
//! let subscription = CallbackDisposal::new(|| {}); // Step 2, built with the lock released.
//! assert!(slot.fill(subscription).is_none()); // Step 3, under the lock: stored.
//!
//! let evicted = slot.reserve_replacing(); // The next swap evicts the stored subscription…
//! assert!(evicted.is_some()); // …to be disposed outside the lock.
//! ```

use educe::Educe;

/// The one inner subscription an operator holds at a time.
///
/// `D` is the value being held — a [`Subscription`](crate::observable::Subscription) in every
/// current use, which disposes when dropped.
#[derive(Educe)]
#[educe(Debug)]
pub enum SubscriptionSlot<D> {
    /// Nothing is subscribed and nothing is being subscribed.
    Idle,
    /// A subscription is being built with the lock released. Reserving the slot up front is what
    /// tells a concurrent update that a subscription is on its way.
    Reserved,
    /// The subscription was released before its build returned. The reservation stays occupied
    /// until `fill` hands back the built subscription and makes the slot idle.
    ReleasedWhileReserved,
    /// A subscription is held.
    Active(D),
}

impl<D> SubscriptionSlot<D> {
    /// Whether the slot is [`Idle`](Self::Idle).
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Whether a build is still in flight, even if its subscription has already been released.
    pub fn is_reserved(&self) -> bool {
        matches!(self, Self::Reserved | Self::ReleasedWhileReserved)
    }

    /// Reserves the slot for a subscription that is about to be built, and gives back the
    /// subscription it replaces, if any, to drop outside the lock.
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

    /// Fills a reserved slot with the subscription that was built.
    ///
    /// Returns `Some` when the slot was released while the build was running — the operator
    /// terminated the inner subscription in the meantime — in which case the slot becomes idle
    /// and the value is given back to drop outside the lock. If this starts another subscription,
    /// decide what to subscribe to and reserve it under the same lock as this fill.
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

    /// Releases the slot, giving back the held subscription, if any, to drop outside the lock.
    ///
    /// Releasing a reserved slot returns `None` and keeps the reservation occupied until the
    /// pending [`fill`](Self::fill) gives its subscription back. Releasing an active slot makes
    /// it idle and returns its subscription; if this starts another subscription, reserve it
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
