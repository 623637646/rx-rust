mod tests_utils;

use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::utils::on_panic::{OnPanic, on_panic};
use std::sync::{Arc, Mutex};

#[test]
fn test_disarm() {
    let seen = Arc::new(Mutex::new(None));
    let seen_of_action = seen.clone();
    let guard = OnPanic::new(111, move |state| {
        seen_of_action.replace_value(Some(state));
    });

    // The scope ends the returning way, so the state comes back and the action never runs.
    assert_eq!(guard.disarm(), 111);
    assert_eq!(seen.clone_value(), None);
}

#[test]
fn test_dropped_without_panicking() {
    let seen = Arc::new(Mutex::new(None));
    let seen_of_action = seen.clone();

    drop(on_panic(move || {
        seen_of_action.replace_value(Some(111));
    }));

    assert_eq!(seen.clone_value(), None);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panic() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let seen = Arc::new(Mutex::new(None));
    let seen_of_action = seen.clone();
    let token = Arc::new(Mutex::new(None));
    let token_of_scope = token.clone();

    expect_panic_on_drop(|panic_on_drop| {
        token.replace_value(Some(panic_on_drop));
        let _guard = OnPanic::new(111, move |state| {
            seen_of_action.replace_value(Some(state));
        });
        // Dropping the token panics, which unwinds out of the guarded scope.
        drop(token_of_scope.take_value());
    });

    // The action ran on the unwinding thread, with the state the guard was carrying.
    assert_eq!(seen.take_value(), Some(111));
}
