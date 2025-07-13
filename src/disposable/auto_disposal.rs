use crate::{
    disposable::{Disposable, boxed_disposal::BoxedDisposal},
    utils::types::NecessarySend,
};

pub struct AutoDisposal<'dis>(Option<BoxedDisposal<'dis>>);

impl<'dis> AutoDisposal<'dis> {
    pub fn new(disposal: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(Some(BoxedDisposal::new(disposal)))
    }
}

impl Disposable for AutoDisposal<'_> {
    fn dispose(self) {
        // drop self to call the dispose
    }
}

impl Drop for AutoDisposal<'_> {
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}
