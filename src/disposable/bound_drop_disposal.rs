//! A disposal that disposes when it is dropped: the [`Subscription`](crate::observable::Subscription) type.

use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use educe::Educe;

/// A disposal that disposes the one inside it when it is dropped.
///
/// This is what [`Subscription`](crate::observable::Subscription) is: holding it keeps the
/// subscription alive, dropping it unsubscribes. The inner disposal can only be reached through
/// the combinators below, which rewrap it, so it is never disposed twice.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{bound_drop_disposal::BoundDropDisposal, callback_disposal::CallbackDisposal};
///
/// let mut disposed = false;
/// {
///     let _subscription = BoundDropDisposal::new(CallbackDisposal::new(|| disposed = true));
///     // Still subscribed.
/// } // Dropped here, which disposes.
/// assert!(disposed);
/// ```
#[must_use = "dropping this disposal immediately disposes the inner disposable"]
#[derive(Educe)]
#[educe(Debug)]
pub struct BoundDropDisposal<D: Disposable>(Option<D>);

impl<D: Disposable> BoundDropDisposal<D> {
    /// Wraps `disposal`, to be disposed when the result is dropped.
    pub fn new(disposal: D) -> Self {
        Self(Some(disposal))
    }

    /// Chains `other` in front of the inner disposal, so that it is disposed first.
    pub fn preceded_by<D0: Disposable>(self, other: D0) -> BoundDropDisposal<ChainDisposal<D0, D>> {
        BoundDropDisposal::new(ChainDisposal::new(other, self.into_inner()))
    }

    /// Chains `other` after the inner disposal, so that it is disposed second.
    pub fn then<D1: Disposable>(self, other: D1) -> BoundDropDisposal<ChainDisposal<D, D1>> {
        BoundDropDisposal::new(ChainDisposal::new(self.into_inner(), other))
    }

    /// [`preceded_by`](Self::preceded_by) for another bound disposal, unwrapping it so that
    /// the result holds both and neither disposes on its own.
    pub fn preceded_by_bound<D1: Disposable>(
        self,
        other: BoundDropDisposal<D1>,
    ) -> BoundDropDisposal<ChainDisposal<D1, D>> {
        BoundDropDisposal::new(ChainDisposal::new(other.into_inner(), self.into_inner()))
    }

    /// Converts the inner disposal with [`From`], typically into a type made by
    /// [`delegate_disposal!`](crate::delegate_disposal).
    pub fn map_into<D1>(self) -> BoundDropDisposal<D1>
    where
        D1: From<D> + Disposable,
    {
        BoundDropDisposal::new(self.into_inner().into())
    }

    // Private: an inner disposal that escaped would be disposed by its taker and, without the
    // `take`, by this drop as well.
    fn into_inner(mut self) -> D {
        self.0.take().unwrap()
    }
}

impl Default for BoundDropDisposal<()> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<D: Disposable> Disposable for BoundDropDisposal<D> {
    fn dispose(self) {
        // Drop to call the dispose
    }
}

impl<D: Disposable> Drop for BoundDropDisposal<D> {
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}
