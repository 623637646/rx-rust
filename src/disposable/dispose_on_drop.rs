//! A wrapper that disposes its inner resource when dropped.

use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use educe::Educe;

/// A wrapper that calls the inner disposal's [`Disposable::dispose`] when dropped.
///
/// [`Observable::subscribe`](crate::observable::Observable::subscribe) returns this wrapper for a
/// subscription; [`Scheduler::run_task`](crate::scheduler::Scheduler::run_task) returns it for a
/// task. Dropping it unsubscribes or cancels the task. Calling [`dispose`](Disposable::dispose)
/// consumes the wrapper and releases the resource immediately.
///
/// The combinators below move the inner disposal into a new wrapper without disposing it during
/// the transfer.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{dispose_on_drop::DisposeOnDrop, callback_disposal::CallbackDisposal};
///
/// let mut disposed = false;
/// {
///     let _disposal = DisposeOnDrop::new(CallbackDisposal::new(|| disposed = true));
///     // The callback has not run yet.
/// } // Dropped here, which disposes.
/// assert!(disposed);
/// ```
#[must_use = "dropping this disposal immediately disposes the inner disposable"]
#[derive(Educe)]
#[educe(Debug)]
pub struct DisposeOnDrop<D: Disposable>(Option<D>);

impl<D: Disposable> DisposeOnDrop<D> {
    /// Wraps `disposal`, to be disposed when the result is dropped.
    pub fn new(disposal: D) -> Self {
        Self(Some(disposal))
    }

    /// Chains `other` in front of the inner disposal, so that it is disposed first.
    pub fn preceded_by<D0: Disposable>(self, other: D0) -> DisposeOnDrop<ChainDisposal<D0, D>> {
        DisposeOnDrop::new(ChainDisposal::new(other, self.into_inner()))
    }

    /// Chains `other` after the inner disposal, so that it is disposed second.
    pub fn then<D1: Disposable>(self, other: D1) -> DisposeOnDrop<ChainDisposal<D, D1>> {
        DisposeOnDrop::new(ChainDisposal::new(self.into_inner(), other))
    }

    /// Chains another [`DisposeOnDrop`] in front, unwrapping both inner disposals.
    ///
    /// Neither wrapper disposes during the transfer. Dropping or disposing the result disposes
    /// `other`'s inner disposal first, then this one's.
    pub fn preceded_by_wrapped<D1: Disposable>(
        self,
        other: DisposeOnDrop<D1>,
    ) -> DisposeOnDrop<ChainDisposal<D1, D>> {
        DisposeOnDrop::new(ChainDisposal::new(other.into_inner(), self.into_inner()))
    }

    /// Converts the inner disposal with `f`, wrapping the result to dispose it on drop.
    ///
    /// Unlike [`map_inner_into`](Self::map_inner_into) it needs no `From`, which is what erasing the disposal
    /// into a box takes.
    pub fn map_inner<D1: Disposable>(self, f: impl FnOnce(D) -> D1) -> DisposeOnDrop<D1> {
        DisposeOnDrop::new(f(self.into_inner()))
    }

    /// Converts the inner disposal with [`From`], typically into a type made by
    /// [`delegate_disposal!`](crate::delegate_disposal).
    pub fn map_inner_into<D1>(self) -> DisposeOnDrop<D1>
    where
        D1: From<D> + Disposable,
    {
        DisposeOnDrop::new(self.into_inner().into())
    }

    // Keep extraction private so callers transfer the inner disposal through the combinators,
    // which return another wrapper that disposes on drop.
    fn into_inner(mut self) -> D {
        self.0.take().unwrap()
    }
}

impl Default for DisposeOnDrop<()> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<D: Disposable> Disposable for DisposeOnDrop<D> {
    fn dispose(self) {
        // Dropping `self` disposes the inner disposal.
    }
}

impl<D: Disposable> Drop for DisposeOnDrop<D> {
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}
