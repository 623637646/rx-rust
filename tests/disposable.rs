mod tests_utils;

use rx_rust::subscription::disposable::{BoxedDisposal, CallbackDisposal, Disposable};
use tests_utils::test_struct::TestStruct;

#[test]
fn test_callback_disposal() {
    let mut called = false;
    let disposal = CallbackDisposal::new(|| {
        called = true;
    });
    disposal.dispose();
    assert!(called);
}

#[test]
fn test_boxed_disposal() {
    let mut called = false;
    let disposal = CallbackDisposal::new(|| {
        called = true;
    });
    let disposal = BoxedDisposal::new(disposal);
    disposal.dispose();
    assert!(called);
}

#[test]
fn test_lifetime_boxed() {
    // OK
    let life_marker = TestStruct;
    let disposal;

    // Error
    // let disposal;
    // let life_marker = TestStruct;

    {
        let callback_disposal = CallbackDisposal::new(|| {
            life_marker.consume_ref();
        });
        disposal = BoxedDisposal::new(callback_disposal);
    }

    _ = disposal;
}
