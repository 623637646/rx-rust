mod tests_utils;

use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::{Disposable, DisposableExt, dispose_on_drop::DisposeOnDrop};
use rx_rust::thread_mode::mutable::{MutableBoolHelper, MutableHelper};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tests_utils::test_struct::TestStruct;

struct TestDisposal {
    disposed: Arc<AtomicBool>,
}

impl Disposable for TestDisposal {
    fn dispose(self) {
        assert!(!self.disposed.read());
        self.disposed.write(true);
    }
}

#[test]
fn test_disposal_dispose() {
    let disposed = Arc::new(AtomicBool::new(false));
    let test_disposal = TestDisposal {
        disposed: disposed.clone(),
    };
    let disposal = DisposeOnDrop::new(test_disposal);
    assert!(!disposed.read());
    disposal.dispose();
    assert!(disposed.read());
}

#[test]
fn test_disposal_dropped() {
    let disposed = Arc::new(AtomicBool::new(false));
    {
        let test_disposal = TestDisposal {
            disposed: disposed.clone(),
        };
        let _disposal = DisposeOnDrop::new(test_disposal);
        assert!(!disposed.read());
    }
    assert!(disposed.read());
}

#[test]
fn test_callback_dispose() {
    let disposed = Arc::new(AtomicBool::new(false));
    let disposed_clone = disposed.clone();
    let disposal = DisposeOnDrop::new(CallbackDisposal::new(move || {
        assert!(!disposed_clone.read());
        disposed_clone.write(true);
    }));
    assert!(!disposed.read());
    disposal.dispose();
    assert!(disposed.read());
}

#[test]
fn test_callback_dropped() {
    let disposed = Arc::new(AtomicBool::new(false));
    {
        let disposed_clone = disposed.clone();
        let _disposal = DisposeOnDrop::new(CallbackDisposal::new(move || {
            assert!(!disposed_clone.read());
            disposed_clone.write(true);
        }));
        assert!(!disposed.read());
    }
    assert!(disposed.read());
}

#[test]
fn test_chain_disposable() {
    let disposed_1 = Arc::new(AtomicBool::new(false));
    let disposed_2 = Arc::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let disposal = DisposeOnDrop::new(test_disposal_1).preceded_by(test_disposal_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    disposal.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
}

#[test]
fn test_chain_disposable_then() {
    let disposed_1 = Arc::new(AtomicBool::new(false));
    let disposed_2 = Arc::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let disposal = DisposeOnDrop::new(test_disposal_1).then(test_disposal_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    disposal.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
}

#[test]
fn test_chain_wrapped_disposals() {
    let disposed_1 = Arc::new(AtomicBool::new(false));
    let disposed_2 = Arc::new(AtomicBool::new(false));
    let test_disposal_1 = TestDisposal {
        disposed: disposed_1.clone(),
    };
    let test_disposal_2 = TestDisposal {
        disposed: disposed_2.clone(),
    };
    let disposal_1 = DisposeOnDrop::new(test_disposal_1);
    let disposal_2 = DisposeOnDrop::new(test_disposal_2);
    let disposal = disposal_1.preceded_by_wrapped(disposal_2);
    assert!(!disposed_1.read());
    assert!(!disposed_2.read());
    disposal.dispose();
    assert!(disposed_1.read());
    assert!(disposed_2.read());
}

#[test]
fn test_chain_disposal_order() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let callback = |value: u8| {
        let order = order.clone();
        CallbackDisposal::new(move || order.with_mut(|order| order.push(value)))
    };
    let disposal = DisposeOnDrop::new(callback(2))
        .preceded_by(callback(1))
        .then(callback(3))
        .preceded_by_wrapped(DisposeOnDrop::new(callback(0)));

    order.with_ref(|order| assert!(order.is_empty()));
    drop(disposal);
    order.with_ref(|order| assert_eq!(order, &[0, 1, 2, 3]));
}

#[test]
fn test_conversion_preserves_disposal() {
    rx_rust::delegate_disposal!(Converted<D>, D, where D: Disposable);

    let disposed = Arc::new(AtomicBool::new(false));
    let disposal: DisposeOnDrop<Converted<_>> = TestDisposal {
        disposed: disposed.clone(),
    }
    .into_dispose_on_drop();
    assert!(!disposed.read());

    let disposal = disposal.map_inner(DisposableExt::into_boxed);
    assert!(!disposed.read());
    let disposal = disposal.map_inner_into::<Converted<_>>();
    assert!(!disposed.read());

    drop(disposal);
    assert!(disposed.read());
}

#[test]
fn test_lifetime_dis() {
    // OK
    let life_marker = TestStruct;
    let _disposal;

    // Error
    // let _disposal;
    // let life_marker = TestStruct;

    {
        let callback = || {
            life_marker.consume_ref();
        };
        _disposal = DisposeOnDrop::new(CallbackDisposal::new(callback));
    }
}
