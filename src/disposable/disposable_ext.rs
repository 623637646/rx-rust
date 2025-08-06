use crate::{
    disposable::Disposable,
    safe_lock_option_disposable,
    utils::types::{Mutable, Shared},
};

impl<D> Disposable for Shared<Mutable<Option<D>>>
where
    D: Disposable,
{
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self);
    }
}

#[cfg(feature = "futures")]
impl Disposable for futures::stream::AbortHandle {
    fn dispose(self) {
        self.abort();
    }
}
