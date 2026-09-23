//! A disposal that runs a closure.

use crate::disposable::Disposable;
use educe::Educe;

/// A disposal that runs `callback` once, when it is disposed.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{callback_disposal::CallbackDisposal, Disposable};
///
/// let mut released = false;
/// CallbackDisposal::new(|| released = true).dispose();
/// assert!(released);
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct CallbackDisposal<F: FnOnce()>(#[educe(Debug(ignore))] F);

impl<F: FnOnce()> CallbackDisposal<F> {
    /// Creates a disposal that runs `callback` when disposed.
    pub fn new(callback: F) -> Self {
        Self(callback)
    }
}
impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self) {
        self.0();
    }
}
