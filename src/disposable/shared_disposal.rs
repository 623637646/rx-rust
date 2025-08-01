use crate::{
    disposable::Disposable,
    safe_lock_option_disposable,
    utils::types::{Mutable, Shared},
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
        safe_lock_option_disposable!(dispose: self.0);
    }
}
