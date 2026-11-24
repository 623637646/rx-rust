//! What the unit tests of the groups share.

use super::Events;

impl<Ev> Events<Ev> {
    /// The events emitted, in order.
    pub(in crate::operators::transforming::internal) fn into_vec(self) -> Vec<Ev> {
        match self {
            Self::Zero => Vec::new(),
            Self::One(event) => vec![event],
            Self::Many(events) => events,
        }
    }
}
