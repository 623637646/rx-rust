mod tests_utils;

use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::utils::types::Shared;
use rx_rust::{disposable::Disposable, observable::Subscription};
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
    let subscription = Subscription::new(test_disposal);
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
        let _subscription = Subscription::new(test_disposal);
        assert!(!disposed.load(Ordering::SeqCst));
    }
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_callback_unsubscribe() {
    let disposed = Shared::new(AtomicBool::new(false));
    let disposed_clone = disposed.clone();
    let subscription = Subscription::new(CallbackDisposal::new(move || {
        assert!(!disposed_clone.load(Ordering::SeqCst));
        disposed_clone.store(true, Ordering::SeqCst);
    }));
    assert!(!disposed.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_callback_dropped() {
    let disposed = Shared::new(AtomicBool::new(false));
    {
        let disposed_clone = disposed.clone();
        let _subscription = Subscription::new(CallbackDisposal::new(move || {
            assert!(!disposed_clone.load(Ordering::SeqCst));
            disposed_clone.store(true, Ordering::SeqCst);
        }));
        assert!(!disposed.load(Ordering::SeqCst));
    }
    assert!(disposed.load(Ordering::SeqCst));
}

#[test]
fn test_chain_disposable() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription = Subscription::new(test_disposal_1).preceded_by(test_disposal_2);
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
}

#[test]
fn test_chain_disposable_after_creation() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription = Subscription::new(test_disposal_1).preceded_by(test_disposal_2);
    assert!(!disposed_1.load(Ordering::SeqCst));
    assert!(!disposed_2.load(Ordering::SeqCst));
    subscription.dispose();
    assert!(disposed_1.load(Ordering::SeqCst));
    assert!(disposed_2.load(Ordering::SeqCst));
}

#[test]
fn test_chain_subscription() {
    let disposed_1 = Shared::new(AtomicBool::new(false));
    let disposed_2 = Shared::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription_1 = Subscription::new(test_disposal_1);
    let subscription_2 = Subscription::new(test_disposal_2);
    let subscription = subscription_1.preceded_by_bound(subscription_2);
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
        _subscription = Subscription::new(CallbackDisposal::new(callback));
    }
}
