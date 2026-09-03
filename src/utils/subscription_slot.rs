//! The slot that holds the one inner subscription an operator keeps at a time.
//!
//! An operator such as [`Switch`](crate::operators::combining::switch::Switch) or
//! [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) is subscribed to at most one
//! inner observable at a time, and swaps that subscription as the source emits. Subscribing is an
//! external API call, so it must happen with no lock held, which splits every swap into two
//! locked steps around an unlocked one: reserve the slot, subscribe, fill the slot. Between the
//! two steps the inner observable can terminate the operator synchronously and release the slot,
//! so the fill has to be able to give the new subscription back.
//!
//! [`SubscriptionSlot`] is that three-step state machine and nothing else. It carries **no lock of
//! its own**: it lives inside a model already guarded by the delivery lock — see
//! [`SubscriptionContext::update`](crate::utils::subscribe_with_context::SubscriptionContext::update)
//! — and every method takes `&mut self`. It also disposes nothing: each method hands the
//! subscription it evicts back to the caller, which passes it to
//! [`UpdateOutcome::with_drop_outside`](crate::utils::serialized_delivery::UpdateOutcome::with_drop_outside)
//! so it is dropped outside the lock.
//!
//! # When a slot is the right type
//!
//! [`Reserved`](SubscriptionSlot::Reserved) is the whole of what this type adds. `Idle` and
//! `Active` are what an `Option<D>` already says, so a host that needs only those two keeps its
//! `Option`. Reach for a slot only when the host must tell "a value is on its way" apart from
//! "nothing is held", which takes all three of:
//!
//! - the value is built by an external call that has to run with the lock released, so the state
//!   is observable by someone else while the build is in flight;
//! - the slot can be released inside that window, and filling a released slot would install a
//!   value that is already dead — in a host that keeps one task alive while there is work to do,
//!   that means storing the handle of a task that already stopped, after which no new task is
//!   ever started and the queued events stall;
//! - there is exactly one such value. Several of them are keyed, and an absent key already means
//!   `Idle`, so each entry collapses back to `Option` — see the maps in
//!   [`MergeAll`](crate::operators::combining::merge_all::MergeAll) and
//!   [`Amb`](crate::operators::conditional_boolean::amb::Amb), which run this same reserve/fill
//!   protocol without this type.
//!
//! When the third state is unreachable, a slot only widens the state space with a variant the
//! host's invariants forbid, which is the opposite of what it is for.
//!
//! # Why this is not [`SharedDisposal`](crate::disposable::shared_disposal::SharedDisposal)
//!
//! The two are the same shape — idle, building, active — but not the same contract, so neither is
//! built on the other:
//!
//! - `SharedDisposal` is itself a disposal, so it needs a terminal `Disposed` state that absorbs
//!   later replacements. A slot needs none: its host stops through the delivery, which stops
//!   running updates at all ([`DeliveryStopped`](crate::utils::serialized_delivery::DeliveryStopped)),
//!   and whatever is left in the slot is disposed when the model is dropped.
//! - `SharedDisposal` releases its own lock while the builder runs, so a second `replace` can race
//!   the first and it needs a generation id to tell whether a finished build is still current. A
//!   slot cannot be raced: the reserve/fill pair is serialized by the delivery lock, and
//!   [`Reserved`](SubscriptionSlot::Reserved) makes a second reserve unreachable.
//!
//! Merging them would give every slot a state it never enters and an id it never reads.

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
    /// A subscription is held.
    Active(D),
}

impl<D> SubscriptionSlot<D> {
    /// Whether the slot is [`Idle`](Self::Idle).
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Whether the slot is [`Reserved`](Self::Reserved).
    pub fn is_reserved(&self) -> bool {
        matches!(self, Self::Reserved)
    }

    /// Reserves the slot for a subscription that is about to be built, and gives back the
    /// subscription it replaces, if any, to drop outside the lock.
    ///
    /// # Panics
    ///
    /// Panics if the slot is already reserved: only one build can be in flight, since the caller
    /// reserves under the same lock that serializes its updates.
    pub fn reserve(&mut self) -> Option<D> {
        match std::mem::replace(self, Self::Reserved) {
            Self::Idle => None,
            Self::Active(value) => Some(value),
            Self::Reserved => unreachable!("the slot is already reserved"),
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
            Self::Reserved | Self::Active(_) => false,
        }
    }

    /// Fills a reserved slot with the subscription that was built.
    ///
    /// Returns `Some` when the slot was released while the build was running — the operator
    /// terminated the inner subscription in the meantime — in which case the value was not stored
    /// and is given back to drop outside the lock.
    ///
    /// # Panics
    ///
    /// Panics if the slot is already active, which would mean a build was never reserved.
    pub fn fill(&mut self, value: D) -> Option<D> {
        match self {
            Self::Reserved => {
                *self = Self::Active(value);
                None
            }
            Self::Idle => Some(value),
            Self::Active(_) => unreachable!("the slot was filled without being reserved"),
        }
    }

    /// Releases the slot, giving back the held subscription, if any, to drop outside the lock.
    ///
    /// Releasing a [`Reserved`](Self::Reserved) slot returns `None` and makes the pending
    /// [`fill`](Self::fill) give its subscription back instead of storing it.
    pub fn release(&mut self) -> Option<D> {
        match std::mem::replace(self, Self::Idle) {
            Self::Active(value) => Some(value),
            Self::Idle | Self::Reserved => None,
        }
    }
}
