//! A lock together with a flag that is raised only under it, and read without it.
//!
//! What the flag means is up to the owner of the lock — a delivery that has stopped, an inbox
//! that the next value must not skip — but the guarantee is the same: the flag is raised by
//! [`FlaggedLock::raise_with`], under the lock, together with whatever its callback changes, so
//! code that reads it raised and then takes the lock finds that change. Code that reads it lowered
//! takes no lock at all, which is the point: the common path of a state that is rarely raised —
//! the drop of a delivery's handle, each value `observe_on` delivers — costs one read of the
//! flag, and does not contend with the threads that take the lock.
//!
//! The flag is only ever raised (a [`RaiseOnlyFlag`]), and the lock and the flag are private to
//! this module, so nothing can raise it but a [`raise_with`](FlaggedLock::raise_with), which holds
//! the lock.

use crate::thread_mode::{
    ThreadMode,
    mutable::{MutableHelper, RaiseOnlyFlag},
};
use educe::Educe;

/// A value behind the lock the thread mode `M` picks, and a flag raised only under that lock.
/// Clones share both.
#[derive(Educe)]
#[educe(Clone(bound()))]
pub(crate) struct FlaggedLock<M: ThreadMode, T> {
    value: M::Ptr<T>,
    raised: M::Flag,
}

impl<M: ThreadMode, T> FlaggedLock<M, T> {
    /// Locks `value`, with the flag lowered.
    pub(crate) fn new(value: T) -> Self {
        Self {
            value: M::ptr(value),
            raised: M::Flag::default(),
        }
    }

    /// Runs `callback` on the value, under the lock, leaving the flag as it is.
    pub(crate) fn with_mut<Out>(&self, callback: impl FnOnce(&mut T) -> Out) -> Out {
        self.value.with_mut(callback)
    }

    /// Raises the flag and runs `callback` on the value, both under the lock: the only way to
    /// raise it. The flag is raised before `callback` runs, so what it changes is seen by whoever
    /// reads the flag raised and then locks, and code that reads the flag while what `callback`
    /// returns is dropped, after the lock, sees it raised.
    pub(crate) fn raise_with<Out>(&self, callback: impl FnOnce(&mut T) -> Out) -> Out {
        self.value.with_mut(|value| {
            self.raised.raise();
            callback(value)
        })
    }

    /// Runs `callback` on the value, under the lock, once the flag has been raised, and takes no
    /// lock before: [`None`] until then. So it may be called anywhere, under this very lock
    /// included, as long as the flag is lowered there.
    pub(crate) fn if_raised<Out>(&self, callback: impl FnOnce(&mut T) -> Out) -> Option<Out> {
        if !self.raised.is_raised() {
            return None;
        }
        Some(self.value.with_mut(callback))
    }

    /// Creates a view of the flag, which reads it without the lock and cannot raise it.
    pub(crate) fn view(&self) -> FlagView<M> {
        FlagView(self.raised.clone())
    }
}

/// Reads the flag of a [`FlaggedLock`] without its lock.
pub(crate) struct FlagView<M: ThreadMode>(M::Flag);

impl<M: ThreadMode> FlagView<M> {
    /// Whether the flag has been raised, for good.
    pub(crate) fn is_raised(&self) -> bool {
        self.0.is_raised()
    }
}

#[cfg(test)]
mod tests {
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
}
