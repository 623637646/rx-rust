use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use educe::Educe;

/// A disposal that calls the `dispose` method of a `Disposable` when dropped.
#[derive(Educe)]
#[educe(Debug)]
pub struct BoundDropDisposal<D: Disposable>(Option<D>);

impl<D: Disposable> BoundDropDisposal<D> {
    pub fn new(disposal: D) -> Self {
        Self(Some(disposal))
    }

    pub fn preceded_by<D0: Disposable>(self, other: D0) -> BoundDropDisposal<ChainDisposal<D0, D>> {
        BoundDropDisposal::new(ChainDisposal::new(other, self.into_inner()))
    }

    pub fn then<D1: Disposable>(self, other: D1) -> BoundDropDisposal<ChainDisposal<D, D1>> {
        BoundDropDisposal::new(ChainDisposal::new(self.into_inner(), other))
    }

    pub fn preceded_by_bound<D1: Disposable>(
        self,
        other: BoundDropDisposal<D1>,
    ) -> BoundDropDisposal<ChainDisposal<D1, D>> {
        BoundDropDisposal::new(ChainDisposal::new(other.into_inner(), self.into_inner()))
    }

    pub fn map_into<D1>(self) -> BoundDropDisposal<D1>
    where
        D1: From<D> + Disposable,
    {
        BoundDropDisposal::new(self.into_inner().into())
    }

    // Private for safety
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
