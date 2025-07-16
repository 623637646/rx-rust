use crate::{
    disposable::{
        Disposable, binding_drop_disposal::BindingDropDisposal, boxed_disposal::BoxedDisposal,
        callback_disposal::CallbackDisposal,
    },
    utils::types::NecessarySend,
};
use std::ops::Add;

/// Subscription is from Observable pattern, it is used to unsubscribe the observable.
/// The `dispose` method of `Disposable` will be called when the subscription is unsubscribe or dropped.
pub struct Subscription<'dis>(Vec<BindingDropDisposal<BoxedDisposal<'dis>>>);

impl<'dis> Subscription<'dis> {
    /// Create a new `Subscription` with no disposal. No action will be performed when the subscription is unsubscribed or dropped.
    pub fn new_none_disposal() -> Self {
        Self(vec![])
    }

    pub fn new_with_disposal(disposable: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(vec![BindingDropDisposal::new(BoxedDisposal::new(
            disposable,
        ))])
    }

    pub fn new_with_disposal_callback(callback: impl FnOnce() + NecessarySend + 'dis) -> Self {
        Self(vec![BindingDropDisposal::new(BoxedDisposal::new(
            CallbackDisposal::new(callback),
        ))])
    }

    pub fn append_disposable(&mut self, disposable: impl Disposable + NecessarySend + 'dis) {
        self.0
            .push(BindingDropDisposal::new(BoxedDisposal::new(disposable)));
    }
}

impl Disposable for Subscription<'_> {
    fn dispose(self) {
        // drop self to call the dispose
    }
}

impl<'dis, T> Add<T> for Subscription<'dis>
where
    T: Disposable + NecessarySend + 'dis,
{
    type Output = Subscription<'dis>;

    #[inline]
    fn add(mut self, other: T) -> Subscription<'dis> {
        self.append_disposable(other);
        self
    }
}
