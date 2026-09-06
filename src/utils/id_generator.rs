//! Ids handed out under a lock, to tell one thing apart from another that replaced it.
//!
//! Two shapes use this, and the difference lives in the caller, not in the id:
//!
//! - a *generation*, compared against [`IdGenerator::latest`] to tell whether something built with
//!   the lock released is still the current one — see
//!   [`SharedDisposal`](crate::disposable::shared_disposal::SharedDisposal) and
//!   [`Switch`](crate::operators::combining::switch::Switch);
//! - a *key*, identifying an entry of a collection — see
//!   [`MergeAll`](crate::operators::combining::merge_all::MergeAll) and
//!   [`PublishSubject`](crate::subject::publish_subject::PublishSubject).
//!
//! An [`Id`] can only come from an [`IdGenerator`], which never hands the same one out twice, so a
//! holder of an id cannot forge one that collides with a later value.

/// Hands out an [`Id`] that is never equal to any it handed out before.
///
/// It carries no lock of its own: it lives inside state that is already guarded, and
/// [`next_id`](Self::next_id) takes `&mut self`.
#[derive(Debug, Default)]
pub struct IdGenerator(u64);

/// An id handed out by an [`IdGenerator`].
///
/// There is deliberately no `Default`: an id can only be obtained from a generator, so it can
/// never accidentally equal the value a generator starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

impl IdGenerator {
    /// The next id, greater than every id handed out before.
    ///
    /// `u64` is big enough that this never wraps on any platform.
    #[must_use = "the id identifies what is being handed out, and is the only way to recognize it later"]
    pub fn next_id(&mut self) -> Id {
        self.0 += 1;
        Id(self.0)
    }

    /// The id handed out last, or `None` before the first one.
    pub fn latest(&self) -> Option<Id> {
        (self.0 != 0).then_some(Id(self.0))
    }
}
