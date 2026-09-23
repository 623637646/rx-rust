//! The [`Mutable`] lock — `RefCell` in the single-threaded build, `Mutex` otherwise — and the
//! only sanctioned way to reach through it: a callback that gets a reference and returns.
//!
//! # Rule 1: the guard must not outlive the operation
//!
//! Two guards of the same lock alive in one statement deadlock a `Mutex`
//! (<https://stackoverflow.com/q/79621758/9315497>):
//!
//! ```ignore
//! let equals = lock.lock().unwrap().clone() == lock.lock().unwrap().clone(); // Deadlock
//! ```
//!
//! [`MutableHelper::with_mut`] and [`with_ref`](MutableHelper::with_ref) rule this out
//! structurally: the callback gets a `&mut T` / `&T`, and the guard is released before they
//! return, so it can never be named, stored or compared.
//!
//! # Rule 2: the callback must not run anything that can take the same lock again
//!
//! This is the rule the type system cannot enforce. Calling an [`Observer`](crate::observer::Observer)
//! or a [`Disposable`](crate::disposable::Disposable) from the callback is user code that can
//! re-enter the operator, and so is *dropping* a value: `subscriptions.with_mut(Vec::clear)`
//! deadlocks `merge_all`, because dropping a subscription disposes it, which takes this very lock.
//!
//! The fix is always the same shape — **take the value out under the lock, act on it afterwards**:
//!
//! - [`MutableExt::take_value`] / [`MutableExt::replace_value`] hand the displaced value back, so
//!   the caller drops it outside the lock;
//! - when the decision itself needs the lock, return an action from the callback and run it after
//!   `with_mut` returns, as `ref_count`, `unicast_subject` and `serialized_delivery` do.
//!
//! Debug builds check the rule at runtime: taking a lock the current thread already holds panics
//! at the offending call site instead of deadlocking.
//!
//! # Panics and poisoning
//!
//! Callback panics propagate normally. The `Mutex` backend recovers the guard from a poison
//! error so that later access, including cleanup during unwinding, does not panic merely because
//! the lock is poisoned. This neither clears the poison flag nor repairs or rolls back the value:
//! callbacks must leave state suitable for subsequent access and cleanup if they panic.
//!
//! # Examples
//! ```rust
//! use rx_rust::utils::mutable::{Mutable, MutableExt, MutableHelper};
//!
//! let counter = Mutable::new(vec![1, 2, 3]);
//! let sum = counter.with_ref(|values| values.iter().sum::<i32>());
//! assert_eq!(sum, 6);
//!
//! counter.with_mut(|values| values.push(4));
//! let taken = counter.take_value(); // Drop the values outside the lock.
//! assert_eq!(taken, [1, 2, 3, 4]);
//! assert!(counter.with_ref(Vec::is_empty));
//! ```

mod reentrancy;

/// The single entry point to a [`Mutable`], for both backends.
///
/// The callback gets a plain reference, never the guard, so the lock cannot be held past the
/// callback, and whatever the callback returns lives — and is dropped — outside it. See the
/// [module documentation](self) for what the callback must not do.
pub trait MutableHelper {
    /// The guarded value.
    type Value;

    /// Runs `callback` with exclusive access to the value, releasing the lock before returning.
    fn with_mut<R>(&self, callback: impl FnOnce(&mut Self::Value) -> R) -> R;
    /// Runs `callback` with shared access to the value, releasing the lock before returning.
    fn with_ref<R>(&self, callback: impl FnOnce(&Self::Value) -> R) -> R;
}

/// The handful of one-shot operations that cover most uses of a [`Mutable`].
///
/// Each of them takes the lock exactly once and hands every value it produces back to the caller,
/// so the value is used — and dropped — after the lock has been released.
///
/// The `_value` suffixes keep these names clear of the inherent methods of the two backends:
/// `RefCell::take` already exists, and `Mutex::{get_cloned, set}` exist behind the unstable
/// `lock_value_accessors` feature. An inherent method wins method resolution over a trait one, so
/// a colliding name would silently bypass the re-entrancy check on one of the two backends.
pub trait MutableExt: MutableHelper {
    /// Clones the contained value.
    fn clone_value(&self) -> Self::Value
    where
        Self::Value: Clone,
    {
        self.with_ref(Clone::clone)
    }

    /// Stores `value` and returns the replaced one, leaving it to the caller to drop it outside
    /// the lock.
    fn replace_value(&self, value: Self::Value) -> Self::Value {
        self.with_mut(|current| std::mem::replace(current, value))
    }

    /// Takes the contained value out, leaving the default in its place.
    ///
    /// For a `Mutable<Option<T>>` this is `Option::take`, and for a `Mutable<Vec<T>>` it is the
    /// deadlock-free replacement for `clear`: the elements are dropped by the caller instead of
    /// under the lock.
    ///
    /// This is the shape to reach for whenever what follows is anything the crate does not own —
    /// `Observer::on_next`, `Observer::on_termination`, `Disposable::dispose` — because all of
    /// them can re-enter the very lock they were reached through. `slot.take_value().map(f)` runs
    /// `f` with the lock already released.
    fn take_value(&self) -> Self::Value
    where
        Self::Value: Default,
    {
        self.with_mut(std::mem::take)
    }
}

impl<M: MutableHelper + ?Sized> MutableExt for M {}

/// A lock-free flag: [`MutableBool`] is a `Cell<bool>` in the single-threaded build and an
/// `AtomicBool` otherwise.
pub trait MutableBoolHelper {
    /// The current value.
    fn read(&self) -> bool;
    /// Sets the value.
    fn write(&self, value: bool);
    /// Sets the value to `value` and returns whether that changed it.
    fn change_if_not_equal(&self, value: bool) -> bool;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::cell::{Cell, RefCell};

        /// The lock: a `RefCell` in the single-threaded build, a `Mutex` otherwise. Reach through
        /// it with [`MutableHelper`] and [`MutableExt`] only.
        pub type Mutable<T> = RefCell<T>;

        impl<T> MutableHelper for RefCell<T> {
            type Value = T;

            fn with_mut<R>(&self, callback: impl FnOnce(&mut T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&mut self.borrow_mut())
            }
            fn with_ref<R>(&self, callback: impl FnOnce(&T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&self.borrow())
            }
        }

        /// The flag: a `Cell<bool>` in the single-threaded build, an `AtomicBool` otherwise.
        pub type MutableBool = Cell<bool>;
        impl MutableBoolHelper for Cell<bool> {
            fn read(&self) -> bool {
                self.get()
            }
            fn write(&self, value: bool) {
                self.set(value)
            }
            fn change_if_not_equal(&self, value: bool) -> bool {
                let old = self.replace(value);
                old != value
            }
        }
    } else {
        use std::sync::{Mutex, PoisonError};
        use std::sync::atomic::{AtomicBool, Ordering};

        /// The lock: a `Mutex` in the multi-threaded build, a `RefCell` otherwise. Reach through
        /// it with [`MutableHelper`] and [`MutableExt`] only.
        pub type Mutable<T> = Mutex<T>;

        impl<T> MutableHelper for Mutex<T> {
            type Value = T;

            fn with_mut<R>(&self, callback: impl FnOnce(&mut T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&mut self.lock().unwrap_or_else(PoisonError::into_inner))
            }
            fn with_ref<R>(&self, callback: impl FnOnce(&T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&self.lock().unwrap_or_else(PoisonError::into_inner))
            }
        }

        /// The flag: an `AtomicBool` in the multi-threaded build, a `Cell<bool>` otherwise.
        pub type MutableBool = AtomicBool;
        impl MutableBoolHelper for MutableBool {
            fn read(&self) -> bool {
                self.load(Ordering::SeqCst)
            }
            fn write(&self, value: bool) {
                self.store(value, Ordering::SeqCst)
            }
            fn change_if_not_equal(&self, value: bool) -> bool {
                self.compare_exchange(!value, value, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            }
        }
    }
}
