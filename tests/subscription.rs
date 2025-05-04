mod tests_utils;

use rx_rust::subscription::{Subscription, disposable::Disposable};
use std::sync::{Arc, RwLock};
use tests_utils::test_struct::TestStruct;

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
        let _subscription = Subscription::new_with_disposal(test_disposal);
        assert!(!*disposed.read().unwrap());
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
        let _subscription = Subscription::new_with_disposal_callback(move || {
            let mut disposed = disposed_clone.write().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        assert!(!*disposed.read().unwrap());
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
fn test_lifetime_dis() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let callback = || {
            life_marker.consume_ref();
        };
        _subscription = Subscription::new_with_disposal_callback(callback);
    }
}
