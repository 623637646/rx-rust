//! Ids handed out under a lock, to tell one thing apart from another that replaced it.
//!
//! Two shapes use this, and the difference lives in the caller, not in the id:
//!
//! - a *generation*, kept by the state and compared against the one a build was handed, to tell
//!   whether something built with the lock released is still the current one — see
//!   [`ReplaceableDisposal`](crate::disposable::replaceable_disposal::ReplaceableDisposal);
//! - a *key*, identifying an entry of a collection — see
//!   [`ReservationMap`](crate::utils::reservation_map::ReservationMap) and the observers of
//!   [`SerializedMulticast`](crate::utils::serialized_multicast::SerializedMulticast).
//!
//! An [`Id`] can only come from an [`IdGenerator`], which never hands the same one out twice, so a
//! holder of an id cannot forge one that collides with a later value.

/// Hands out an [`Id`] that is never equal to any it handed out before.
///
/// It carries no lock of its own: it lives inside state that is already guarded, and
/// [`next_id`](Self::next_id) takes `&mut self`.
#[derive(Debug, Default)]
pub(crate) struct IdGenerator(u64);

/// An id handed out by an [`IdGenerator`].
///
/// There is deliberately no `Default`: an id can only be obtained from a generator, so it can
/// never accidentally equal the value a generator starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Id(u64);

impl IdGenerator {
    /// The next id, greater than every id handed out before.
    ///
    /// `u64` is big enough that this never wraps on any platform.
    #[must_use = "the id identifies what is handed out, and is the only way to recognize it later"]
    pub(crate) fn next_id(&mut self) -> Id {
        self.0 += 1;
        Id(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::IdGenerator;

    #[test]
    fn test_next_id() {
        let mut ids = IdGenerator::default();
        let first = ids.next_id();
        let second = ids.next_id();
        assert!(first < second);
        assert_ne!(first, second); // `first` is never handed out again.
    }
}
