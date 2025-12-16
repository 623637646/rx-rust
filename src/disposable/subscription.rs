use crate::{
    disposable::{
        Disposable, bound_drop_disposal::BoundDropDisposal, boxed_disposal::BoxedDisposal,
        callback_disposal::CallbackDisposal,
    },
    utils::types::NecessarySend,
};
use educe::Educe;
use std::ops::Add;

/// Subscription is from Observable pattern, it is used to unsubscribe the observable.
/// The `dispose` method of `Disposable` will be called when the subscription is unsubscribe or dropped.
#[derive(Educe)]
#[educe(Debug, Default)]
pub struct Subscription<'dis>(Vec<BoundDropDisposal<BoxedDisposal<'dis>>>);

impl<'dis> Subscription<'dis> {
    pub fn new() -> Self {
        Self(Vec::default())
    }

    pub fn new_with_disposal(disposable: impl Disposable + NecessarySend + 'dis) -> Self {
        Self(vec![BoundDropDisposal::new(BoxedDisposal::new(disposable))])
    }

    pub fn new_with_disposal_callback(callback: impl FnOnce() + NecessarySend + 'dis) -> Self {
        Self(vec![BoundDropDisposal::new(BoxedDisposal::new(
            CallbackDisposal::new(callback),
        ))])
    }

    pub fn append_disposable(&mut self, disposable: impl Disposable + NecessarySend + 'dis) {
        self.0
            .push(BoundDropDisposal::new(BoxedDisposal::new(disposable)));
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
