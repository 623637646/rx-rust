//! A disposal that may hold nothing.

use crate::disposable::Disposable;
use educe::Educe;

/// A disposal that either holds a disposal to dispose or nothing at all.
///
/// An operator that sometimes never subscribes to its source — `take(0)`, or a `start_with` whose
/// prepended values already ended the stream — returns this, so that both paths share one type.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{callback_disposal::CallbackDisposal, option_disposal::OptionDisposal, Disposable};
///
/// let mut disposed = false;
/// OptionDisposal::some(CallbackDisposal::new(|| disposed = true)).dispose();
/// assert!(disposed);
///
/// OptionDisposal::<CallbackDisposal<fn()>>::none().dispose(); // Nothing to release.
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OptionDisposal<D>(Option<D>);

impl<D> OptionDisposal<D> {
    /// Wraps `disposal`, which is disposed when this one is.
    pub fn some(disposal: D) -> Self {
        Self(Some(disposal))
    }

    /// A disposal that releases nothing.
    pub fn none() -> Self {
        Self(None)
    }
}

impl<D: Disposable> Disposable for OptionDisposal<D> {
    fn dispose(self) {
        if let Some(disposal) = self.0 {
            Disposable::dispose(disposal)
        }
    }
}
