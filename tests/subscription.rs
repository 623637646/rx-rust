mod tests_utils;

use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::utils::types::Shared;
use rx_rust::utils::types::{MutableBool, MutableBoolHelper};
use rx_rust::{disposable::Disposable, observable::Subscription};
use tests_utils::test_struct::TestStruct;

struct TestDisposal {
    disposed: Shared<MutableBool>,
}

impl Disposable for TestDisposal {
    fn dispose(self) {
        assert!(!self.disposed.read());
        self.disposed.write(true);
    }
}

#[test]
fn test_disposal_unsubscribe() {
    let disposed = Shared::new(MutableBool::new(false));
    let test_disposal = TestDisposal {
        disposed: disposed.clone(),
    };
    let subscription = Subscription::new(test_disposal);
    assert!(!disposed.read());
    subscription.dispose();
    assert!(disposed.read());
}

#[test]
fn test_disposal_dropped() {
    let disposed = Shared::new(MutableBool::new(false));
    {
        let test_disposal = TestDisposal {
            disposed: disposed.clone(),
        };
        let _subscription = Subscription::new(test_disposal);
        assert!(!disposed.read());
    }
    assert!(disposed.read());
}

#[test]
fn test_callback_unsubscribe() {
    let disposed = Shared::new(MutableBool::new(false));
    let disposed_clone = disposed.clone();
    let subscription = Subscription::new(CallbackDisposal::new(move || {
        assert!(!disposed_clone.read());
        disposed_clone.write(true);
    }));
    assert!(!disposed.read());
    subscription.dispose();
    assert!(disposed.read());
}

#[test]
fn test_callback_dropped() {
    let disposed = Shared::new(MutableBool::new(false));
    {
        let disposed_clone = disposed.clone();
        let _subscription = Subscription::new(CallbackDisposal::new(move || {
            assert!(!disposed_clone.read());
            disposed_clone.write(true);
        }));
        assert!(!disposed.read());
    }
    assert!(disposed.read());
}

#[test]
fn test_chain_disposable() {
    let disposed_1 = Shared::new(MutableBool::new(false));
    let disposed_2 = Shared::new(MutableBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription = Subscription::new(test_disposal_1).preceded_by(test_disposal_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    subscription.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
}

#[test]
fn test_chain_disposable_after_creation() {
    let disposed_1 = Shared::new(MutableBool::new(false));
    let disposed_2 = Shared::new(MutableBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription = Subscription::new(test_disposal_1).preceded_by(test_disposal_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    subscription.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
}

#[test]
fn test_chain_subscription() {
    let disposed_1 = Shared::new(MutableBool::new(false));
    let disposed_2 = Shared::new(MutableBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let subscription_1 = Subscription::new(test_disposal_1);
    let subscription_2 = Subscription::new(test_disposal_2);
    let subscription = subscription_1.preceded_by_bound(subscription_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    subscription.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
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
