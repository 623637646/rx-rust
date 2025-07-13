use crate::disposable::Disposable;

/// A disposal that calls a callback when disposed.
pub struct CallbackDisposal<F: FnOnce()>(F);

impl<F: FnOnce()> CallbackDisposal<F> {
    /// Creates a new callback disposal.
    pub fn new(callback: F) -> Self {
        Self(callback)
    }
}

impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self) {
        self.0();
    }
}
