mod tests_utils;

use rx_rust::disposable::{Disposable, subscription::Subscription};
use rx_rust::utils::types::Shared;
use std::sync::atomic::{AtomicBool, Ordering};
use tests_utils::test_struct::TestStruct;

struct TestDisposal {
    disposed: Shared<AtomicBool>,
}
impl Disposable for TestDisposal {
    fn dispose(self) {
        assert!(!self.disposed.load(Ordering::SeqCst));
        self.disposed.store(true, Ordering::SeqCst);
    }
}

#[test]
fn test_disposal_unsubscribe() {
    let disposed = Shared::new(AtomicBool::new(false));
    let test_disposal = TestDisposal {
        disposed: disposed.clone(),
    };
    let subscription = Subscription::new_with_disposal(test_disposal);
    assert!(!disposed.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_disposal_dropped() {
    let disposed = Shared::new(AtomicBool::new(false));
    {
        let test_disposal = TestDisposal {
            disposed: disposed.clone(),
        };
        let _subscription = Subscription::new_with_disposal(test_disposal);
        assert!(!disposed.load(Ordering::SeqCst));
    }
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_callback_unsubscribe() {
    let disposed = Shared::new(AtomicBool::new(false));
    let disposed_clone = disposed.clone();
    let subscription = Subscription::new_with_disposal_callback(move || {
        assert!(!disposed_clone.load(Ordering::SeqCst));
        disposed_clone.store(true, Ordering::SeqCst);
    });
    assert!(!disposed.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_callback_dropped() {
    let disposed = Shared::new(AtomicBool::new(false));
    {
        let disposed_clone = disposed.clone();
        let _subscription = Subscription::new_with_disposal_callback(move || {
            assert!(!disposed_clone.load(Ordering::SeqCst));
            disposed_clone.store(true, Ordering::SeqCst);
        });
        assert!(!disposed.load(Ordering::SeqCst));
    }
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_append_disposable() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let mut subscription = Subscription::new_with_disposal(test_disposal_1);
    subscription.append_disposable(test_disposal_2);
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
}

#[test]
fn test_add_disposable() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription = Subscription::new_with_disposal(test_disposal_1);
    let subscription = subscription + test_disposal_2;
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
}

#[test]
fn test_add_subscription() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription_1 = Subscription::new_with_disposal(test_disposal_1);
    let subscription_2 = Subscription::new_with_disposal(test_disposal_2);
    let subscription = subscription_1 + subscription_2;
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
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
