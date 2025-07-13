mod tests_utils;

use rx_rust::disposable::{
    Disposable, boxed_disposal::BoxedDisposal, callback_disposal::CallbackDisposal,
};
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
    let _disposal;

    // Error
    // let _disposal;
    // let life_marker = TestStruct;

    {
        let callback_disposal = CallbackDisposal::new(|| {
            life_marker.consume_ref();
        });
        _disposal = BoxedDisposal::new(callback_disposal);
    }
}
