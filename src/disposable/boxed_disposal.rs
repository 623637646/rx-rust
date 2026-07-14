use crate::{disposable::Disposable, utils::types::MaybeSend};

trait ErasedDisposable {
    fn dispose_boxed(self: Box<Self>);
}

impl<D> ErasedDisposable for D
where
    D: Disposable,
{
    fn dispose_boxed(self: Box<Self>) {
        Disposable::dispose(*self);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Type-erased disposal for single-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedDisposal<'dis>(Box<dyn ErasedDisposable + 'dis>);
    } else {
        /// Type-erased disposal for multi-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedDisposal<'dis>(Box<dyn ErasedDisposable + Send + 'dis>);
    }
}

impl<'dis> BoxedDisposal<'dis> {
    pub fn new(disposal: impl Disposable + MaybeSend + 'dis) -> Self {
        Self(Box::new(disposal))
    }
}

impl Disposable for BoxedDisposal<'_> {
    #[inline]
    fn dispose(self) {
        self.0.dispose_boxed();
    }
}

impl std::fmt::Debug for BoxedDisposal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
