pub mod disposable;

use disposable::{BoxedDisposal, CallbackDisposal, Disposable};
use std::ops::Add;

/// Subscription is from Observable pattern, it is used to unsubscribe the observable.
/// The `dispose` method of `Disposable` will be called when the subscription is unsubscribe or dropped.
pub struct Subscription<'dis>(Vec<BoxedDisposal<'dis>>);

impl<'dis> Subscription<'dis> {
    /// Create a new subscription.
    pub fn new_with_disposals(disposables: Vec<BoxedDisposal<'dis>>) -> Self {
        Self(disposables)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests_utils::test_struct::TestStruct;
    use std::sync::{Arc, RwLock};

    struct TestDisposal {
        disposed: Arc<RwLock<bool>>,
    }
    impl Disposable for TestDisposal {
        fn dispose(self) {
            let mut disposed = self.disposed.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        }
    }

    #[test]
    fn test_disposal_unsubscribe() {
        let disposed = Arc::new(RwLock::new(false));
        let test_disposal = TestDisposal {
            disposed: disposed.clone(),
        };
        let subscription = Subscription::new_with_disposal(test_disposal);
        assert!(!*disposed.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_disposal_dropped() {
        let disposed = Arc::new(RwLock::new(false));
        {
            let test_disposal = TestDisposal {
                disposed: disposed.clone(),
            };
            let subscription = Subscription::new_with_disposal(test_disposal);
            assert!(!*disposed.read().unwrap());
            _ = subscription; // keep the subscription alive
        }
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_callback_unsubscribe() {
        let disposed = Arc::new(RwLock::new(false));
        let disposed_clone = disposed.clone();
        let subscription = Subscription::new_with_disposal_callback(move || {
            let mut disposed = disposed_clone.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        assert!(!*disposed.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_callback_dropped() {
        let disposed = Arc::new(RwLock::new(false));
        {
            let disposed_clone = disposed.clone();
            let subscription = Subscription::new_with_disposal_callback(move || {
                let mut disposed = disposed_clone.write().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
            assert!(!*disposed.read().unwrap());

            _ = subscription; // keep the subscription alive
        }
        assert!(*disposed.read().unwrap());
    }

    #[test]
    fn test_append_disposable() {
        let disposed_1 = Arc::new(RwLock::new(false));
        let disposed_2 = Arc::new(RwLock::new(false));
        let test_disposal_1 = TestDisposal {
            disposed: disposed_1.clone(),
        };
        let test_disposal_2 = TestDisposal {
            disposed: disposed_2.clone(),
        };
        let mut subscription = Subscription::new_with_disposal(test_disposal_1);
        subscription.append_disposable(test_disposal_2);
        assert!(!*disposed_1.read().unwrap());
        assert!(!*disposed_2.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed_1.read().unwrap());
        assert!(*disposed_2.read().unwrap());
    }

    #[test]
    fn test_add_disposable() {
        let disposed_1 = Arc::new(RwLock::new(false));
        let disposed_2 = Arc::new(RwLock::new(false));
        let test_disposal_1 = TestDisposal {
            disposed: disposed_1.clone(),
        };
        let test_disposal_2 = TestDisposal {
            disposed: disposed_2.clone(),
        };
        let subscription = Subscription::new_with_disposal(test_disposal_1);
        let subscription = subscription + test_disposal_2;
        assert!(!*disposed_1.read().unwrap());
        assert!(!*disposed_2.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed_1.read().unwrap());
        assert!(*disposed_2.read().unwrap());
    }

    #[test]
    fn test_append_subscription() {
        let disposed_1 = Arc::new(RwLock::new(false));
        let disposed_2 = Arc::new(RwLock::new(false));
        let test_disposal_1 = TestDisposal {
            disposed: disposed_1.clone(),
        };
        let test_disposal_2 = TestDisposal {
            disposed: disposed_2.clone(),
        };
        let mut subscription_1 = Subscription::new_with_disposal(test_disposal_1);
        let subscription_2 = Subscription::new_with_disposal(test_disposal_2);
        subscription_1.append_subscription(subscription_2);
        assert!(!*disposed_1.read().unwrap());
        assert!(!*disposed_2.read().unwrap());
        subscription_1.unsubscribe();
        assert!(*disposed_1.read().unwrap());
        assert!(*disposed_2.read().unwrap());
    }

    #[test]
    fn test_add_subscription() {
        let disposed_1 = Arc::new(RwLock::new(false));
        let disposed_2 = Arc::new(RwLock::new(false));
        let test_disposal_1 = TestDisposal {
            disposed: disposed_1.clone(),
        };
        let test_disposal_2 = TestDisposal {
            disposed: disposed_2.clone(),
        };
        let subscription_1 = Subscription::new_with_disposal(test_disposal_1);
        let subscription_2 = Subscription::new_with_disposal(test_disposal_2);
        let subscription = subscription_1 + subscription_2;
        assert!(!*disposed_1.read().unwrap());
        assert!(!*disposed_2.read().unwrap());
        subscription.unsubscribe();
        assert!(*disposed_1.read().unwrap());
        assert!(*disposed_2.read().unwrap());
    }

    #[test]
    fn test_lifetime() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let callback = || {
                life_marker.consume_ref();
            };
            subscription = Subscription::new_with_disposal_callback(callback);
        }
        _ = subscription;
    }
}
