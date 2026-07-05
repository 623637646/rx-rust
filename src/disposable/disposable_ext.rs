use crate::{
    disposable::Disposable,
    safe_lock_option_disposable,
    utils::types::{Mutable, Shared},
};

// TODO: remove this after using SharedDisposal
impl<D> Disposable for Shared<Mutable<Option<D>>>
where
    D: Disposable,
{
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self);
    }
}
