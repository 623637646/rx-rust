mod tests_utils;

use rx_rust::disposable::boxed_disposal::SendBoxedDisposal;
use rx_rust::disposable::{Disposable, callback_disposal::CallbackDisposal};
use tests_utils::test_struct::TestStruct;

#[cfg(panic = "unwind")]
#[test]
fn test_chain_disposes_second_on_panic() {
    use rx_rust::disposable::chain_disposal::ChainDisposal;
    use tests_utils::panic::expect_panic_on_drop;

    let mut disposed = false;
    expect_panic_on_drop(|panic_on_drop| {
        ChainDisposal::new(
            CallbackDisposal::new(|| drop(panic_on_drop)),
            CallbackDisposal::new(|| disposed = true),
        )
        .dispose();
    });
    assert!(disposed);
}

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
    let disposal = SendBoxedDisposal::new(disposal);
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
        _disposal = SendBoxedDisposal::new(callback_disposal);
    }
}
