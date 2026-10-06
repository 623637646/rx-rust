//! Records which locks the current thread holds, so that taking one of them again — which
//! deadlocks a `Mutex` and panics a `RefCell` — is reported at the offending call site instead.
//!
//! Debug builds only; [`held_lock`] compiles to nothing otherwise.
//!
//! A lock is identified by its address and its size. The address alone is not enough: a lock
//! nested in another one — a `Mutex<RefCell<_>>`, or a state whose first field is a lock — may sit
//! at the very address of the lock around it, and taking the two in turn would read as a
//! re-entry. The outer lock holds the inner one plus its own bookkeeping, so it is always the
//! larger of the two.

#[cfg(debug_assertions)]
mod enabled {
    use std::cell::RefCell;

    thread_local! {
        static HELD: RefCell<Vec<LockId>> = const { RefCell::new(Vec::new()) };
    }

    /// The address and the size of a lock.
    pub(super) type LockId = (usize, usize);

    /// Marks the lock `id` as held for as long as this guard lives.
    pub(crate) struct HeldLock(LockId);

    impl HeldLock {
        pub(super) fn enter(id: LockId) -> Self {
            // The check runs outside the borrow so that the assertion below cannot unwind while
            // `HELD` is borrowed, which would make every `HeldLock::drop` on the way out panic in
            // turn.
            let already_held = HELD
                .try_with(|held| held.borrow().contains(&id))
                .unwrap_or(false);
            assert!(
                !already_held,
                "this lock is already held by the current thread: the callback of a `with_mut` / \
                 `with_ref` must not take the same lock again. Take the value out of the lock and \
                 act on it afterwards - see the `mutable` module docs."
            );
            let _ = HELD.try_with(|held| held.borrow_mut().push(id));
            Self(id)
        }
    }

    impl Drop for HeldLock {
        fn drop(&mut self) {
            // Searched rather than popped, so that an unusual drop order cannot leave a stale
            // entry behind and report a later, unrelated lock as re-entered.
            let _ = HELD.try_with(|held| {
                let held = &mut *held.borrow_mut();
                if let Some(index) = held.iter().rposition(|id| *id == self.0) {
                    held.remove(index);
                }
            });
        }
    }
}

#[cfg(debug_assertions)]
pub(super) fn held_lock<T>(lock: &T) -> enabled::HeldLock {
    enabled::HeldLock::enter((std::ptr::from_ref(lock) as usize, size_of::<T>()))
}

#[cfg(not(debug_assertions))]
pub(super) fn held_lock<T>(_lock: &T) {}
