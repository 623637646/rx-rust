/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self: Box<Self>);
}

/// A disposal that calls a callback when disposed.
pub struct CallbackDisposal<F: FnOnce()>(F);

impl<F> CallbackDisposal<F>
where
    F: FnOnce(),
{
    /// Creates a new callback disposal.
    pub fn new(callback: F) -> CallbackDisposal<F> {
        CallbackDisposal(callback)
    }
}

impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self: Box<Self>) {
        self.0();
    }
}
