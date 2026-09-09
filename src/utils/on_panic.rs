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

use educe::Educe;

/// Runs `action` with the guarded state if the current scope unwinds, and nothing otherwise.
///
/// See the [module documentation](self) for what the action may do. Use [`on_panic`] when there is
/// no state to carry.
#[must_use = "a guard that is not bound to a variable is dropped at once, guarding nothing"]
#[derive(Educe)]
#[educe(Debug)]
pub struct OnPanic<T, F: FnOnce(T)>(Option<(T, F)>);

impl<T, F: FnOnce(T)> OnPanic<T, F> {
    /// Arms the guard, parking `state` in it until the scope ends one way or the other.
    pub fn new(state: T, action: F) -> Self {
        Self(Some((state, action)))
    }

    /// Ends the guarded scope the returning way, handing the state back untouched.
    pub fn disarm(mut self) -> T {
        let (state, _action) = self.0.take().expect("the guard is disarmed at most once");
        state
    }
}

impl<T, F: FnOnce(T)> Drop for OnPanic<T, F> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        if let Some((state, action)) = self.0.take() {
            action(state);
        }
    }
}

/// Runs `action` if the current scope unwinds, for a guard that carries no state.
pub fn on_panic<F: FnOnce()>(action: F) -> OnPanic<(), impl FnOnce(())> {
    OnPanic::new((), move |()| action())
}
