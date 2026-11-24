//! The groups of the buffer operators: [`Bundles`], kept in the state of the machine that decides
//! when they open and close, under its lock, and emitted as they close (decision 0014).
//!
//! Putting an item into a bundle runs no user code, so the bundles, unlike the
//! [`Windows`](super::window::Windows), need no delegate outside the lock: an item is pushed in the
//! one update the machine takes anyway, and only a finished bundle is an event.

use super::{group::Groups, overlap::Overlap};
use crate::observer::{Flow, Observer, Termination};

/// The bundles of a buffer operator, the oldest first: each one is emitted as it closes, and those
/// left open that are not empty as the source completes.
///
/// A bundle is made with room for as many items as the last bundle emitted had, so that a stream
/// of bundles of one length builds each with a single allocation; before the first is emitted,
/// with the room the operator gives (the `count` of an operator with one). A count operator's
/// bundles all close full, so their room stays `count`. A bundle is made as it opens, before the
/// bundle before it is freed, as `take_leaving_room` does: the allocator then never sees its memory
/// go back and forth.
#[doc(hidden)]
pub struct Bundles<T> {
    /// The open bundles, the oldest first.
    bundles: Overlap<Vec<T>, T>,
    /// The room a bundle is made with: the length of the last bundle emitted.
    room: usize,
}

impl<T> Bundles<T> {
    /// No bundle open yet, the first ones made with room for `room` items. `duplicate` clones an
    /// item for every open bundle but the newest.
    pub(crate) fn with_room(duplicate: fn(&T) -> T, room: usize) -> Self {
        Self {
            bundles: Overlap::new(duplicate),
            room,
        }
    }
}

impl<T, E> Groups<E> for Bundles<T> {
    type Item = T;
    type Event = Vec<T>;

    fn open(&mut self, _: &mut impl Observer<Vec<T>, E>) -> Flow {
        self.bundles.open(Vec::with_capacity(self.room));
        Flow::Continue(())
    }

    #[inline(always)]
    fn put(&mut self, value: &mut Option<T>, _: &mut impl Observer<Vec<T>, E>) -> Flow {
        self.bundles.send(value, Vec::push);
        Flow::Continue(())
    }

    fn close_oldest(&mut self, observer: &mut impl Observer<Vec<T>, E>) -> Flow {
        let Some(bundle) = self.bundles.close_oldest() else {
            return Flow::Continue(());
        };
        self.room = bundle.len();
        observer.on_next(bundle)
    }

    fn end(
        &mut self,
        termination: &Termination<E>,
        observer: &mut impl Observer<Vec<T>, E>,
    ) -> Flow {
        match termination {
            // The bundles left after a stop are dropped with the iterator.
            Termination::Completed => self
                .bundles
                .close_all()
                .filter(|bundle| !bundle.is_empty())
                .try_for_each(|bundle| observer.on_next(bundle)),
            // An error discards the bundles: they are dropped with the state, outside any lock.
            Termination::Error(_) => Flow::Continue(()),
        }
    }
}

#[cfg(test)]
mod tests;
