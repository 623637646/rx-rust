use super::FlaggedLock;
use crate::thread_mode::{Local, Shared};

#[test]
fn test_starts_lowered() {
    let lock = FlaggedLock::<Local, _>::new(0);

    assert!(!lock.view().is_raised());
    assert_eq!(lock.if_raised(|_| unreachable!()), None::<()>);
}

#[test]
fn test_with_mut_leaves_the_flag_lowered() {
    let lock = FlaggedLock::<Local, _>::new(0);

    lock.with_mut(|value| *value = 1);

    assert!(!lock.view().is_raised());
    assert_eq!(lock.with_mut(|value| *value), 1);
}

#[test]
fn test_raise_with_raises_the_flag_before_its_callback() {
    let lock = FlaggedLock::<Local, _>::new(0);
    let view = lock.view();

    let seen_raised = lock.raise_with(|value| {
        // Reading the flag takes no lock, even while the raising callback holds it.
        assert!(lock.is_raised());
        *value = 1;
        view.is_raised()
    });

    assert!(seen_raised);
    assert!(view.is_raised());
    assert_eq!(lock.if_raised(|value| *value), Some(1));
}

#[test]
fn test_raising_again_keeps_the_flag_raised() {
    let lock = FlaggedLock::<Local, _>::new(0);

    lock.raise_with(|value| *value = 1);
    lock.raise_with(|value| *value = 2);

    assert!(lock.view().is_raised());
    assert_eq!(lock.if_raised(|value| *value), Some(2));
}

#[test]
fn test_if_raised_takes_no_lock_while_lowered() {
    let lock = FlaggedLock::<Local, _>::new(0);

    // Under the lock already: taking it again would panic, as the `RefCell` is borrowed (and
    // debug builds check reentrancy too), so this passes only if no lock is taken.
    let inner = lock.with_mut(|_| lock.if_raised(|_| unreachable!()));

    assert_eq!(inner, None::<()>);
}

#[test]
fn test_clones_share_the_value_and_the_flag() {
    let lock = FlaggedLock::<Shared, _>::new(0);
    let clone = lock.clone();
    let view = lock.view();

    std::thread::scope(|scope| {
        scope.spawn(|| clone.raise_with(|value| *value = 1));
    });

    assert!(view.is_raised());
    assert_eq!(lock.if_raised(|value| *value), Some(1));
}
