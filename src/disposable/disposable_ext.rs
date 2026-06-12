use crate::{
    disposable::Disposable,
    safe_lock_option_disposable,
    utils::types::{Mutable, MutableBool, MutableBoolHelper, Shared},
};

impl<D> Disposable for Shared<Mutable<Option<D>>>
where
    D: Disposable,
{
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self);
    }
}

impl Disposable for Shared<MutableBool> {
    fn dispose(self) {
        self.write(false);
    }
}

#[cfg(feature = "futures")]
impl Disposable for futures::stream::AbortHandle {
    fn dispose(self) {
        self.abort();
    }
}
