use crate::{
    disposable::Disposable,
    utils::{
        safe_lock::SafeLockOption,
        types::{Mutable, Shared},
    },
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
        self.0.safe_lock_dispose_if_some();
    }
}
