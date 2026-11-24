//! What the unit tests of the source sets share.

use crate::observer::{Flow, Observer, Termination};

/// Takes every event and does nothing with it.
pub(super) struct IgnoreObserver;

impl<T, E> Observer<T, E> for IgnoreObserver {
    fn on_next(&mut self, _: T) -> Flow {
        Flow::Continue(())
    }

    fn on_termination(self, _: Termination<E>) {}
}
