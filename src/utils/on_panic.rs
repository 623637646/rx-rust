//! The guard that undoes what an unwinding callback would otherwise leave behind.
//!
//! Everything an operator keeps on its stack — a subscription, an observer, a queued value — is
//! released by the unwind itself, so it needs no guard. What the unwind does not undo is a change
//! already written to a **shared state that outlives the panicking call**: a delivery left in its
//! delivering state, a subscriber counted into a ref count, a source left subscribed after its
//! termination. Those states are reached by writing them *before* the callback and finishing the
//! transaction *after* it, which is exactly the step a panic skips.
//!
//! [`OnPanic`] runs that finishing step on the unwinding thread instead. Its action therefore
//! runs under the constraints of a `Drop` during a panic:
//!
//! - **It must not panic.** A panic while panicking aborts the process. It may drop values, and
//!   run the ordinary disposals that dropping them entails, but it should not call further into
//!   code that is free to unwind.
//! - **It must not take a lock the panicking thread already holds**, which in this crate means it
//!   is only ever wrapped around callbacks that run with no lock held — observer notifications and
//!   external subscriptions. A guard that has to lock is safe exactly where the returning path
//!   locks too.
//!
//! The guard is armed for the scope it is bound to, so it must be bound: `let _guard = …`, and
//! `drop(guard)` where the scope ends before the enclosing block. When the returning path needs
//! something back from the guard, park it in the guard's state and take it with
//! [`OnPanic::disarm`], which ends the scope and hands the state over.
//!
//! A guard armed while its thread is already unwinding — by a `Drop` that runs during the unwind,
//! disposing a subscription that delivers something — guards a scope that panic did not start in,
//! so it never runs its action: `std::thread::panicking` stays `true` through the whole unwind,
//! and would otherwise mistake the earlier panic for one of its own scope. A panic inside that
//! scope aborts the process anyway.

use educe::Educe;

/// Runs `action` with the guarded state if the current scope unwinds, and nothing otherwise.
///
/// See the [module documentation](self) for what the action may do. Use [`on_panic`] when there is
/// no state to carry.
#[must_use = "a guard that is not bound to a variable is dropped at once, guarding nothing"]
#[derive(Educe)]
#[educe(Debug)]
pub(crate) struct OnPanic<T, F: FnOnce(T)> {
    armed: Option<(T, F)>,
    /// Whether the thread was already unwinding when the guard was armed: see the module
    /// documentation.
    armed_while_panicking: bool,
}

impl<T, F: FnOnce(T)> OnPanic<T, F> {
    /// Arms the guard, parking `state` in it until the scope ends one way or the other.
    pub(crate) fn new(state: T, action: F) -> Self {
        Self {
            armed: Some((state, action)),
            armed_while_panicking: std::thread::panicking(),
        }
    }

    /// The guarded state, for the guarded scope to work with while the guard keeps it.
    pub(crate) fn state(&self) -> &T {
        &self
            .armed
            .as_ref()
            .expect("the guard is armed until it is dropped")
            .0
    }

    /// Ends the guarded scope the returning way, handing the state back untouched.
    pub(crate) fn disarm(mut self) -> T {
        let (state, _action) = self
            .armed
            .take()
            .expect("the guard is disarmed at most once");
        state
    }
}

impl<T, F: FnOnce(T)> Drop for OnPanic<T, F> {
    fn drop(&mut self) {
        if self.armed_while_panicking || !std::thread::panicking() {
            return;
        }
        if let Some((state, action)) = self.armed.take() {
            action(state);
        }
    }
}

/// Runs `action` if the current scope unwinds, for a guard that carries no state.
pub(crate) fn on_panic<F: FnOnce()>(action: F) -> OnPanic<(), impl FnOnce(())> {
    OnPanic::new((), move |()| action())
}

#[cfg(test)]
mod tests {
    use super::{OnPanic, on_panic};
    use std::{
        cell::Cell,
        panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    };

    #[test]
    fn test_disarm() {
        let seen = Cell::new(None);
        let guard = OnPanic::new(111, |state| seen.set(Some(state)));
        assert_eq!(*guard.state(), 111);

        // The scope ends the returning way, so the state comes back and the action never runs.
        assert_eq!(guard.disarm(), 111);
        assert_eq!(seen.get(), None);
    }

    #[test]
    fn test_dropped_without_panicking() {
        let seen = Cell::new(None);

        drop(on_panic(|| seen.set(Some(111))));

        assert_eq!(seen.get(), None);
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_panic() {
        let seen = Cell::new(None);

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = OnPanic::new(111, |state| seen.set(Some(state)));
            // Unwinds out of the guarded scope without running the panic hook, which would print.
            resume_unwind(Box::new(()));
        }));

        assert!(result.is_err());
        // The action ran on the unwinding thread, with the state the guard was carrying.
        assert_eq!(seen.get(), Some(111));
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_armed_while_panicking() {
        /// Arms and drops a guard from its own `Drop`, as a disposal run by an unwind does.
        struct ArmsOnDrop<'a>(&'a Cell<Option<i32>>);

        impl Drop for ArmsOnDrop<'_> {
            fn drop(&mut self) {
                drop(OnPanic::new(111, |state| self.0.set(Some(state))));
            }
        }

        let seen = Cell::new(None);

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _arms_on_drop = ArmsOnDrop(&seen);
            resume_unwind(Box::new(()));
        }));

        assert!(result.is_err());
        // The guard's own scope returned: the unwind it was armed in is not its panic.
        assert_eq!(seen.get(), None);
    }
}
