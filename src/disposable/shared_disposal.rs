use crate::{
    disposable::Disposable,
    utils::types::{Mutable, MutableHelper, Shared},
};

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
