use super::Trampoline;
use crate::thread_mode::{
    Local,
    mutable::{MutableExt, MutableHelper},
};
use std::cell::RefCell;

#[test]
fn test_park_when_idle() {
    let trampoline = Trampoline::<Local, i32>::default();
    assert_eq!(trampoline.park(1), Some(1));
}

#[test]
fn test_run_without_park() {
    let trampoline = Trampoline::<Local, i32>::default();
    let started = RefCell::new(Vec::new());
    trampoline.run(
        || {},
        |round| started.with_mut(|started| started.push(round)),
    );
    assert!(started.with_ref(Vec::is_empty));
    // The run left the trampoline idle.
    assert_eq!(trampoline.park(1), Some(1));
}

#[test]
fn test_run_starts_parked_rounds_in_a_loop() {
    let trampoline = Trampoline::<Local, i32>::default();
    let started = RefCell::new(Vec::new());
    trampoline.run(
        || assert_eq!(trampoline.park(0), None),
        |round| {
            started.with_mut(|started| started.push(round));
            if round < 3 {
                assert_eq!(trampoline.park(round + 1), None);
            }
        },
    );
    assert_eq!(started.take_value(), [0, 1, 2, 3]);
    assert_eq!(trampoline.park(4), Some(4));
}

#[cfg(panic = "unwind")]
#[test]
fn test_unwind_empties_the_trampoline() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let trampoline = Trampoline::<Local, i32>::default();
    let result = catch_unwind(AssertUnwindSafe(|| {
        trampoline.run(
            || {
                assert_eq!(trampoline.park(0), None);
                panic!("subscribe unwinds");
            },
            |_| unreachable!("the parked round is dropped, not started"),
        );
    }));
    assert!(result.is_err());
    assert_eq!(trampoline.park(1), Some(1));
}
