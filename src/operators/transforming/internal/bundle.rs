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
mod tests {
    use super::{
        super::group::{Events, Groups},
        Bundles,
    };
    use crate::observer::{Flow, Observer, Termination};
    use std::convert::Infallible;

    #[test]
    fn test_bundles_with_room() {
        let mut bundles = Bundles::with_room(i32::clone, 3);
        let mut events = Events::new();
        let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(1), &mut events);
        let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(2), &mut events);
        assert!(matches!(events, Events::Zero));

        let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
        let Events::One(first) = events else {
            panic!("one bundle emitted");
        };
        assert_eq!(first, [1, 2]);
        // Opened before any bundle was emitted: the room given.
        assert_eq!(first.capacity(), 3);

        // The completion emits the open bundles that are not empty.
        let mut events = Events::new();
        let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
        let _ = bundles.end(&Termination::<Infallible>::Completed, &mut events);
        assert_eq!(events.into_vec(), [vec![2]]);
    }

    #[test]
    fn test_bundles_room_of_last() {
        let mut bundles = Bundles::with_room(i32::clone, 0);
        let mut events = Events::new();
        let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
        for value in 1..=5 {
            let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(value), &mut events);
        }
        let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
        let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(6), &mut events);
        let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
        let emitted = events.into_vec();
        assert_eq!(emitted, [vec![1, 2, 3, 4, 5], vec![6]]);
        assert_eq!(emitted[1].capacity(), 5);
    }

    #[test]
    fn test_bundles_value_between_bundles_left() {
        let mut bundles = Bundles::with_room(i32::clone, 2);
        let mut value = Some(1);
        let mut events = Events::new();
        let _ = Groups::<Infallible>::put(&mut bundles, &mut value, &mut events);
        assert_eq!(value, Some(1));
        assert!(matches!(events, Events::Zero));
    }

    #[test]
    fn test_bundles_end_emits_nothing_after_a_stop() {
        /// Takes one bundle, then stops.
        struct TakeOne(Vec<Vec<i32>>);

        impl Observer<Vec<i32>, Infallible> for TakeOne {
            fn on_next(&mut self, value: Vec<i32>) -> Flow {
                self.0.push(value);
                Flow::Break(())
            }

            fn on_termination(self, _: Termination<Infallible>) {}
        }

        let mut bundles = Bundles::with_room(i32::clone, 2);
        let mut observer = TakeOne(Vec::new());
        let _ = Groups::<Infallible>::open(&mut bundles, &mut observer);
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(1), &mut observer);
        let _ = Groups::<Infallible>::open(&mut bundles, &mut observer);
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(2), &mut observer);
        assert!(
            bundles
                .end(&Termination::Completed, &mut observer)
                .is_break()
        );
        assert_eq!(observer.0, [vec![1, 2]]);
    }
}
