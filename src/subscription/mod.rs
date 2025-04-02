pub mod disposable;

use disposable::{BoxedDisposal, CallbackDisposal, Disposable};

/// Subscription is from Observable pattern, it is used to unsubscribe the observable.
/// The `dispose` method of `Disposable` will be called when the subscription is unsubscribe or dropped.
pub struct Subscription<'a>(Vec<BoxedDisposal<'a>>);

impl<'a> Subscription<'a> {
    /// Create a new subscription.
    pub fn new_with_disposals(disposables: Vec<BoxedDisposal<'a>>) -> Subscription<'a> {
        Subscription(disposables)
    }

    /// Create a new `Subscription` with no disposal. No action will be performed when the subscription is unsubscribed or dropped.
    pub fn new_none_disposal() -> Self {
        Subscription(vec![])
    }

    pub fn new_with_disposal(disposable: impl Disposable + Send + 'a) -> Self {
        Subscription(vec![BoxedDisposal::new(disposable)])
    }

    pub fn new_with_disposal_callback(callback: impl FnOnce() + Send + 'a) -> Self {
        Subscription(vec![BoxedDisposal::new(CallbackDisposal::new(callback))])
    }

    pub fn append_disposable(&mut self, disposable: impl Disposable + Send + 'a) {
        self.0.push(BoxedDisposal::new(disposable));
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

#[cfg(test)]
mod tests {
    use super::*;
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
            drop(subscription); // keep the subscription alive
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

            drop(subscription); // keep the subscription alive
        }
        assert!(*disposed.read().unwrap());
    }
}
