pub mod disposable;

use disposable::{BoxedDisposal, CallbackDisposal, Disposable};
use std::ops::Add;

/// Subscription is from Observable pattern, it is used to unsubscribe the observable.
/// The `dispose` method of `Disposable` will be called when the subscription is unsubscribe or dropped.
pub struct Subscription<'dis>(Vec<BoxedDisposal<'dis>>);

impl<'dis> Subscription<'dis> {
    /// Create a new `Subscription` with no disposal. No action will be performed when the subscription is unsubscribed or dropped.
    pub fn new_none_disposal() -> Self {
        Self(vec![])
    }

    pub fn new_with_disposal(disposable: impl Disposable + Send + 'dis) -> Self {
        Self(vec![BoxedDisposal::new(disposable)])
    }

    pub fn new_with_disposal_callback(callback: impl FnOnce() + Send + 'dis) -> Self {
        Self(vec![BoxedDisposal::new(CallbackDisposal::new(callback))])
    }

    pub fn append_disposable(&mut self, disposable: impl Disposable + Send + 'dis) {
        self.0.push(BoxedDisposal::new(disposable));
    }

    pub fn append_subscription(&mut self, mut subscription: Self) {
        self.0.append(&mut subscription.0);
    }

    /// Unsubscribe the subscription.
    pub fn unsubscribe(self) {
        // drop self to call the dispose
    }
}

impl Drop for Subscription<'_> {
    fn drop(&mut self) {
        for disposable in self.0.drain(..) {
            disposable.dispose();
        }
    }
}

impl<'dis, T> Add<T> for Subscription<'dis>
where
    T: Disposable + Send + 'dis,
{
    type Output = Subscription<'dis>;

    #[inline]
    fn add(mut self, other: T) -> Subscription<'dis> {
        self.append_disposable(other);
        self
    }
}

impl<'dis> Add<Subscription<'dis>> for Subscription<'dis> {
    type Output = Subscription<'dis>;

    #[inline]
    fn add(mut self, other: Subscription<'dis>) -> Subscription<'dis> {
        self.append_subscription(other);
        self
    }
}
