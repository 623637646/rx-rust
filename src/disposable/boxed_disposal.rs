use crate::{disposable::Disposable, utils::types::NecessarySend};

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Type-erased disposal for single-threaded builds to handle this problem https://stackoverflow.com/q/46620790/9315497
        pub struct BoxedDisposal<'dis>(Box<dyn FnOnce() + 'dis>);
    } else {
        /// Type-erased disposal for multi-threaded builds to handle this problem https://stackoverflow.com/q/46620790/9315497
        pub struct BoxedDisposal<'dis>(Box<dyn FnOnce() + Send + 'dis>);
    }
}

impl<'dis> BoxedDisposal<'dis> {
    pub fn new(disposal: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(Box::new(|| {
            disposal.dispose();
        }))
    }
}

impl Disposable for BoxedDisposal<'_> {
    fn dispose(self) {
        self.0();
    }
}
