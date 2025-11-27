use crate::disposable::Disposable;
use educe::Educe;

/// A disposal that calls a callback when disposed.
#[derive(Educe)]
#[educe(Debug)]
pub struct CallbackDisposal<F: FnOnce()>(F);

impl<F: FnOnce()> CallbackDisposal<F> {
    pub fn new(callback: F) -> Self {
        Self(callback)
    }
}
impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self) {
        self.0();
    }
}
