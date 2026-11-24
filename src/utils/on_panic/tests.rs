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
