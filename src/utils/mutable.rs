//! The [`Mutable`] abstraction over the single-threaded and the multi-threaded backend, the
//! handful of traits that are the only sanctioned way to reach through it, and the rules a
//! callback has to follow.
//!
//! # Rule 1: the guard must not outlive the operation
//!
//! Refer to this case: <https://stackoverflow.com/q/79621758/9315497>
//! And this case:
//!
//! ```ignore
//! let lock = Mutex::new("My String".to_owned());
//! // let equals = { lock.lock().unwrap().clone() } == { lock.lock().unwrap().clone() }; // No deadlock
//! let equals = lock.lock().unwrap().clone() == lock.lock().unwrap().clone(); // Deadlock
//! ```
//!
//! Both guards are temporaries of the same statement, so the first one is still alive when the
//! second is taken. [`MutableHelper::with_mut`] and [`with_ref`](MutableHelper::with_ref) rule
//! this out structurally: they hand the callback a `&mut T` / `&T`, keep the guard as a temporary
//! of their own body, and release it before returning. A guard can no longer be named, stored or
//! compared.
//!
//! # Rule 2: the callback must not run anything that can take the same lock again
//!
//! This is the rule the type system cannot enforce, and it is the one that bites. Dropping a value
//! counts as running code: the obvious `clear` on a collection of subscriptions
//!
//! ```ignore
//! self.subscriptions.with_mut(Vec::clear) // Deadlock: dropping a Subscription disposes it,
//!                                         // and disposing it takes this very lock again.
//! ```
//!
//! is a deadlock in the `Disposable` of `merge_all`. So is calling an `Observer` or a `Disposable`
//! from inside the callback, because both are user code that can re-enter the operator.
//!
//! The fix is always the same shape — **take the value out under the lock, act on it afterwards**:
//!
//! - [`MutableExt::take_value`] instead of `clear`, and [`MutableExt::replace_value`] instead of
//!   an assignment: both return the value they displaced, so the caller acts on it — and drops it
//!   — outside the lock.
//! - When the operator has to decide *what* to do while holding the lock, let the callback compute
//!   an action and return it, and run that action after `with_mut` has returned. This is what
//!   `ref_count`, `unicast_subject`, `amb` and `serialized_delivery` do.
//!
//! In debug builds this rule is checked at runtime: taking a lock the current thread already holds
//! panics at the offending call site instead of deadlocking.

mod reentrancy;

/// The single entry point to a [`Mutable`], for both the single-threaded and the multi-threaded
/// backend.
///
/// The callback is handed a plain reference rather than the backend's guard, which is what makes
/// the lock impossible to hold longer than the callback: the guard is a temporary inside
/// `with_mut` / `with_ref` and is released before either returns. Everything the callback produces
/// therefore lives — and is dropped — outside the lock.
///
/// See the [module documentation](self) for the rules a callback has to follow.
pub trait MutableHelper {
    type Value;

    fn with_mut<R>(&self, callback: impl FnOnce(&mut Self::Value) -> R) -> R;
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

pub trait MutableBoolHelper {
    fn read(&self) -> bool;
    fn write(&self, value: bool);
    // Change the contained value to `value`, returns true if it was changed. otherwise false.
    fn change_if_not_equal(&self, value: bool) -> bool;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::cell::{Cell, RefCell};

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
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicBool, Ordering};

        pub type Mutable<T> = Mutex<T>;

        impl<T> MutableHelper for Mutex<T> {
            type Value = T;

            fn with_mut<R>(&self, callback: impl FnOnce(&mut T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&mut self.lock().unwrap())
            }
            fn with_ref<R>(&self, callback: impl FnOnce(&T) -> R) -> R {
                let _held = reentrancy::held_lock(self);
                callback(&self.lock().unwrap())
            }
        }

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
