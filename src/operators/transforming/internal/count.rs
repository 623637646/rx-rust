//! The groups opened every `skip` items, each closed after `count` items: the state machine of
//! [`BufferWithCountAndSkip`](super::super::buffer_with_count_and_skip::BufferWithCountAndSkip)
//! and [`WindowWithCountAndSkip`](super::super::window_with_count_and_skip::WindowWithCountAndSkip),
//! and, with a `skip` equal to `count`, of
//! [`BufferWithCount`](super::super::buffer_with_count::BufferWithCount) and
//! [`WindowWithCount`](super::super::window_with_count::WindowWithCount) (decision 0014).
//!
//! It has a single source and nothing else opening or closing its groups, so it needs no lock: its
//! [`Groups`] emit to the observer itself, and the machine stops at the first hook that answers
//! [`Flow::Stop`].
//!
//! The first group opens as the operator is subscribed, and each next one right after the item
//! that ends the `skip` before it: a window is emitted before its first item, and a buffer, whose
//! bundles emit nothing as they open, leaves out on completion the one still empty.

use super::group::Groups;
use crate::observer::{Flow, Observer, Termination};
use std::num::NonZeroUsize;

/// The observer the count operators give their source: `G` the groups, `OR` the observer they emit
/// to.
#[doc(hidden)]
pub struct CountObserver<G, OR> {
    groups: G,
    observer: OR,
    skip: NonZeroUsize,
    /// The items left before the oldest group closes: `count` for the first, then `skip`, since
    /// the groups open `skip` items apart and all last `count` items.
    items_until_close: usize,
    /// The items left before the next group opens.
    items_until_open: usize,
}

impl<G, OR> CountObserver<G, OR> {
    /// The observer of groups of `count` items opened every `skip` items, with none open yet.
    pub(crate) fn new(groups: G, observer: OR, count: NonZeroUsize, skip: NonZeroUsize) -> Self {
        Self {
            groups,
            observer,
            skip,
            items_until_close: count.get(),
            items_until_open: skip.get(),
        }
    }

    /// Opens the first group, as the operator is subscribed: [`Flow::Stop`] when the observer
    /// stopped on it, and the source must not be subscribed.
    pub(crate) fn open_first<E>(&mut self) -> Flow
    where
        G: Groups<E>,
        OR: Observer<G::Event, E>,
    {
        self.groups.open(&mut self.observer)
    }
}

impl<T, E, G, OR> Observer<T, E> for CountObserver<G, OR>
where
    G: Groups<E, Item = T>,
    OR: Observer<G::Event, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let mut value = Some(value);
        let flow = self.groups.put(&mut value, &mut self.observer);
        // With a `skip` larger than `count`, an item between two groups has nowhere to go, and is
        // dropped here, before the events that follow it.
        drop(value);
        if flow.is_stop() {
            return Flow::Stop;
        }
        self.items_until_close -= 1;
        if self.items_until_close == 0 {
            self.items_until_close = self.skip.get();
            if self.groups.close_oldest(&mut self.observer).is_stop() {
                return Flow::Stop;
            }
        }
        self.items_until_open -= 1;
        if self.items_until_open == 0 {
            self.items_until_open = self.skip.get();
            return self.groups.open(&mut self.observer);
        }
        Flow::Continue
    }

    fn on_termination(self, termination: Termination<E>) {
        let Self {
            mut groups,
            mut observer,
            ..
        } = self;
        // What the groups emitted last ends the stream, so a downstream that stopped on it is not
        // terminated on top of that: it has already ended itself.
        if groups.end(&termination, &mut observer).is_continue() {
            observer.on_termination(termination);
        }
    }
}
