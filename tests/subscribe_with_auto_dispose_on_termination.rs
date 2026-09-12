mod tests_utils;

use rx_rust::{
    disposable::callback_disposal::CallbackDisposal,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::{
        mutable::{MutableBool, MutableBoolHelper},
        subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
        types::Shared,
    },
};
use std::convert::Infallible;

/// An observer running `callback` from inside its `on_termination`.
struct HookOnTerminationObserver<F: FnOnce()>(F);

impl<F: FnOnce()> Observer<i32, Infallible> for HookOnTerminationObserver<F> {
    fn on_next(&mut self, _value: i32) -> Flow {
        Flow::Continue
    }

    fn on_termination(self, _termination: Termination<Infallible>) {
        (self.0)();
    }
}

#[test]
fn test_completed() {
    let is_disposed = Shared::new(MutableBool::default());
    let is_disposed_of_source = is_disposed.clone();
    let mut observer_slot = None;
    let _subscription =
        subscribe_with_auto_dispose_on_termination(HookOnTerminationObserver(|| {}), |observer| {
            observer_slot = Some(observer);
            Subscription::new(CallbackDisposal::new(move || {
                is_disposed_of_source.write(true);
            }))
        });
    assert!(!is_disposed.read());

    observer_slot
        .take()
        .unwrap()
        .on_termination(Termination::Completed);

    // The termination disposes the source instead of waiting for the subscription to be dropped.
    assert!(is_disposed.read());
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_on_termination_disposes_the_source() {
    use crate::tests_utils::panic::expect_panic_on_drop;
    use rx_rust::utils::mutable::{Mutable, MutableExt};

    let is_disposed = Shared::new(MutableBool::default());
    let is_disposed_of_source = is_disposed.clone();
    let token = Shared::new(Mutable::new(None));
    let token_of_observer = token.clone();
    let mut observer_slot = None;
    let _subscription = subscribe_with_auto_dispose_on_termination(
        HookOnTerminationObserver(move || {
            // Dropping the token panics, which unwinds out of this termination.
            drop(token_of_observer.take_value());
        }),
        |observer| {
            observer_slot = Some(observer);
            Subscription::new(CallbackDisposal::new(move || {
                is_disposed_of_source.write(true);
            }))
        },
    );

    expect_panic_on_drop(|panic_on_drop| {
        token.replace_value(Some(panic_on_drop));
        observer_slot
            .take()
            .unwrap()
            .on_termination(Termination::Completed);
    });

    // The source is disposed on the way out of the panic, instead of staying subscribed until the
    // subscription is dropped.
    assert!(is_disposed.read());
}
