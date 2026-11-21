//! The disposal that only raises a flag.

use super::Disposable;
use crate::thread_mode::mutable::RaiseOnlyFlag;
use educe::Educe;

/// Raises a flag when it is disposed, for whoever holds a clone of it to read: an observer that
/// stops delivering (`stop_on_disposal`), a pipe that closes (`unicast_subject`).
/// The flag carries no item or observer type, so the disposal does not extend their lifetimes.
#[derive(Educe)]
#[educe(Debug)]
pub(crate) struct FlagDisposal<F>(#[educe(Debug(ignore))] pub(crate) F);

impl<F: RaiseOnlyFlag> Disposable for FlagDisposal<F> {
    fn dispose(self) {
        self.0.raise();
    }
}
