//! Records which locks the current thread holds, so that taking one of them again — which
//! deadlocks a `Mutex` and panics a `RefCell` — is reported at the offending call site instead.
//!
//! Debug builds only; [`held_lock`] compiles to nothing otherwise.

#[cfg(debug_assertions)]
mod enabled {
    use std::cell::RefCell;

    thread_local! {
        static HELD: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    }

    /// Marks the lock at `address` as held for as long as this guard lives.
    pub struct HeldLock(usize);

    impl HeldLock {
        pub fn enter(address: usize) -> Self {
            // The check runs outside the borrow so that the assertion below cannot unwind while
            // `HELD` is borrowed, which would make every `HeldLock::drop` on the way out panic in
            // turn.
            let already_held = HELD
                .try_with(|held| held.borrow().contains(&address))
                .unwrap_or(false);
            assert!(
                !already_held,
                "this lock is already held by the current thread: the callback of a `with_mut` / \
                 `with_ref` must not take the same lock again. Take the value out of the lock and \
                 act on it afterwards - see the `mutable` module docs."
            );
            let _ = HELD.try_with(|held| held.borrow_mut().push(address));
            Self(address)
        }
    }

    impl Drop for HeldLock {
        fn drop(&mut self) {
            // Searched rather than popped, so that an unusual drop order cannot leave a stale
            // address behind and report a later, unrelated lock as re-entered.
            let _ = HELD.try_with(|held| {
                let held = &mut *held.borrow_mut();
                if let Some(index) = held.iter().rposition(|address| *address == self.0) {
                    held.remove(index);
                }
            });
        }
    }
}

#[cfg(debug_assertions)]
pub(super) fn held_lock<T>(lock: &T) -> enabled::HeldLock {
    enabled::HeldLock::enter(std::ptr::from_ref(lock) as usize)
}

#[cfg(not(debug_assertions))]
pub(super) fn held_lock<T>(_lock: &T) {}
