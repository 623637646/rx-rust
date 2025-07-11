use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};

/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self);
}

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

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        pub struct BoxedDisposal<'dis>(Box<dyn FnOnce() + 'dis>);
    } else {
        /// https://stackoverflow.com/a/56447952/9315497
        pub struct BoxedDisposal<'dis>(Box<dyn FnOnce() + Send + 'dis>);
    }
}

impl<'dis> BoxedDisposal<'dis> {
    pub fn new(disposal: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(Box::new(|| {
            disposal.dispose();
        }))
    }
}

impl Disposable for BoxedDisposal<'_> {
    fn dispose(self) {
        self.0();
    }
}

pub struct AutoDisposal<'dis>(Option<BoxedDisposal<'dis>>);

impl<'dis> AutoDisposal<'dis> {
    pub fn new(disposal: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(Some(BoxedDisposal::new(disposal)))
    }
}

impl Disposable for AutoDisposal<'_> {
    fn dispose(self) {
        // drop self to call the dispose
    }
}

impl Drop for AutoDisposal<'_> {
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}

pub struct SharedDisposal<D>(Shared<Mutable<Option<D>>>);

impl<D> SharedDisposal<D> {
    pub fn new(shared_disposal: Shared<Mutable<Option<D>>>) -> Self {
        Self(shared_disposal)
    }
}

impl<D> Disposable for SharedDisposal<D>
where
    D: Disposable,
{
    fn dispose(self) {
        if let Some(disposal) = { self.0.lock_mut().take() } {
            disposal.dispose();
        }
    }
}

#[cfg(feature = "futures")]
impl Disposable for futures::stream::AbortHandle {
    fn dispose(self) {
        self.abort();
    }
}
