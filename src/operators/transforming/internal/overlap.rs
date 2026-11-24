//! The bundles or windows a buffer or window operator keeps open at once (decision 0014).
//!
//! An operator with a skip or a shift opens a new bundle or window at a fixed step, and closes each
//! one at a fixed length from its opening: they open one after the other and last as long, so they
//! close in the order they opened, and only the oldest can be the next to close. Every item goes to
//! every one open when it arrives, cloned into all of them but the newest, which takes the item
//! itself. [`Overlap`] holds those open sinks — the `Vec`s of a buffer, the senders of a window —
//! and that rule.
//!
//! What sets the operators apart stays with each of them: what closes the oldest (a count, a
//! deadline), what opens the next (a count of items, a timer), whether an empty bundle is opened
//! before its first item, and what a completion does with the ones left open.
//!
//! The operator without a skip or a shift is the same one with the step equal to the length: the
//! oldest closes as the next opens, so no more than one is ever open, and no item is ever cloned.
//! It passes [`no_overlap`] as the duplicate, so that it does not ask its items to be [`Clone`].

use std::collections::VecDeque;

/// The duplicate of an operator that never has more than one sink open, so never clones an item:
/// the one without a skip or a shift.
pub(crate) fn no_overlap<T>(_: &T) -> T {
    unreachable!("a single sink is open at a time, so no item is cloned")
}

/// The open sinks, in opening order.
pub(crate) struct Overlap<S, T> {
    /// The open sinks, the oldest first.
    sinks: VecDeque<S>,
    /// Clones an item for every open sink but the newest: `T::clone`, or [`no_overlap`].
    duplicate: fn(&T) -> T,
}

impl<S, T> Overlap<S, T> {
    /// No sink open yet.
    pub(crate) fn new(duplicate: fn(&T) -> T) -> Self {
        Self {
            sinks: VecDeque::new(),
            duplicate,
        }
    }

    /// Opens `sink` as the newest.
    pub(crate) fn open(&mut self, sink: S) {
        self.sinks.push_back(sink);
    }

    /// Whether no sink is open.
    pub(crate) fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }

    /// Closes the oldest open sink and hands it back.
    pub(crate) fn close_oldest(&mut self) -> Option<S> {
        self.sinks.pop_front()
    }

    /// Closes every open sink and hands them back, the oldest first.
    pub(crate) fn close_all(&mut self) -> impl DoubleEndedIterator<Item = S> + use<S, T> {
        std::mem::take(&mut self.sinks).into_iter()
    }

    /// Sends the item to every open sink with `put`, leaving it in `value` when none is open.
    /// The caller can keep `value` outside a lock so its item is dropped there even if cloning
    /// panics.
    #[inline(always)]
    pub(crate) fn send(&mut self, value: &mut Option<T>, mut put: impl FnMut(&mut S, T)) {
        // The newest taken from the back of the iterator leaves the older ones in it, with no
        // index to check: with one sink open, the common case, the loop below has nothing to do.
        let mut sinks = self.sinks.iter_mut();
        let Some(newest) = sinks.next_back() else {
            return;
        };
        if let Some(item) = value.as_ref() {
            for sink in sinks {
                put(sink, (self.duplicate)(item));
            }
        }
        // Taken only once every clone is made, so that a clone that panics leaves it in `value`.
        if let Some(value) = value.take() {
            put(newest, value);
        }
    }
}

#[cfg(test)]
mod tests;
