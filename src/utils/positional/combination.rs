//! Combinations: one item from every position of a source set, as a tuple or a `Vec`.
//!
//! The public [`Combination`] only selects types: the output after a primary item, and the
//! storage of the latest or queued items at each position. It is public because operator items
//! and disposals name those types, and sealed. What the operators do with them — create the
//! storage for a position count, move or clone a combination out, prepend an item — is in the
//! crate-private subtraits.

use super::tuple::for_each_arity;
use std::collections::VecDeque;

mod sealed {
    pub trait Sealed {}
}

/// One item from every position of a source set: the [`Item`](crate::observable::SourceSet::Item)
/// of every [`SourceSet`](crate::observable::SourceSet).
///
/// Implemented for tuples of one to eight items and for `Vec<T>`, without requiring `Clone`. It
/// selects the types the combining operators keep for it and emit.
///
/// # Examples
/// ```rust
/// use rx_rust::observable::Combination;
/// use std::collections::VecDeque;
///
/// struct NotClone;
///
/// let _: <(NotClone, bool) as Combination>::Latest = (Some(NotClone), None);
/// let _: <Vec<NotClone> as Combination>::Latest = vec![None, Some(NotClone)];
/// let _: <(NotClone, bool) as Combination>::Queues = (VecDeque::new(), VecDeque::new());
/// let _: <Vec<NotClone> as Combination>::Queues = vec![VecDeque::new()];
/// let _: <(NotClone, bool) as Combination>::After<u8> = (1, NotClone, true);
/// let _: <Vec<NotClone> as Combination>::After<u8> = (1, vec![NotClone]);
/// ```
pub trait Combination: sealed::Sealed {
    /// The latest item at each position, `None` until it has one.
    type Latest;
    /// The items at each position waiting for their counterparts.
    type Queues;
    /// The combination after a primary item: a flat tuple, or `(T, Vec<Item>)` for a list.
    type After<T>;
}

/// Places a primary item before the combination.
pub(crate) trait PrependOps: Combination + Sized {
    /// Places `first` before this combination, moving every item into the output.
    fn prepend<T>(self, first: T) -> Self::After<T>;
}

/// Creates the latest-value storage and moves a combination out of it.
pub(crate) trait LatestOps: Combination + Sized {
    /// Creates storage with no values at its `position_count` positions: a tuple's arity, or the
    /// length of a list.
    fn empty(position_count: usize) -> Self::Latest;

    /// Takes every item out, in position order, leaving every position without a value.
    ///
    /// # Panics
    ///
    /// Panics if any position has no value.
    fn take_all(storage: &mut Self::Latest) -> Self;
}

/// A combination that can be cloned out of the latest values.
pub(crate) trait CloneLatest: LatestOps {
    /// Clones every item, in position order, when every position has one. Nothing is cloned
    /// otherwise, so that no clone is dropped here, under the lock guarding the storage.
    fn clone_all(storage: &Self::Latest) -> Option<Self>;
}

/// Creates the queues and moves a combination out of them.
pub(crate) trait QueueOps: Combination + Sized {
    /// Creates `position_count` empty queues: a tuple's arity, or the length of a list.
    fn empty(position_count: usize) -> Self::Queues;

    /// Takes the first item of every queue and counts the queues this leaves empty, in the same
    /// pass.
    ///
    /// # Panics
    ///
    /// Panics if any queue is empty.
    fn pop_all(storage: &mut Self::Queues) -> (Self, usize);
}

/// The combination of each tuple arity.
macro_rules! impl_combination_tuple {
    ($(($position:tt, $item:ident, $($_names:tt)*)),+) => {
        impl<$($item),+> sealed::Sealed for ($($item,)+) {}

        impl<$($item),+> Combination for ($($item,)+) {
            type Latest = ($(Option<$item>,)+);
            type Queues = ($(VecDeque<$item>,)+);
            type After<T> = (T, $($item,)+);
        }

        impl<$($item),+> PrependOps for ($($item,)+) {
            fn prepend<T>(self, first: T) -> Self::After<T> {
                (first, $(self.$position,)+)
            }
        }

        impl<$($item),+> LatestOps for ($($item,)+) {
            fn empty(position_count: usize) -> Self::Latest {
                debug_assert_eq!(position_count, const { [$($position),+].len() });
                ($(None::<$item>,)+)
            }

            fn take_all(storage: &mut Self::Latest) -> Self {
                ($(storage.$position.take().expect("every position has a value"),)+)
            }
        }

        impl<$($item: Clone),+> CloneLatest for ($($item,)+) {
            // The items are bound to the names of their types.
            #[allow(non_snake_case)]
            fn clone_all(storage: &Self::Latest) -> Option<Self> {
                match storage {
                    ($(Some($item),)+) => Some(($($item.clone(),)+)),
                    _ => None,
                }
            }
        }

        impl<$($item),+> QueueOps for ($($item,)+) {
            fn empty(position_count: usize) -> Self::Queues {
                debug_assert_eq!(position_count, const { [$($position),+].len() });
                ($(VecDeque::<$item>::new(),)+)
            }

            fn pop_all(storage: &mut Self::Queues) -> (Self, usize) {
                let items =
                    ($(storage.$position.pop_front().expect("every position has a queued value"),)+);
                (items, 0 $(+ usize::from(storage.$position.is_empty()))+)
            }
        }
    };
}

for_each_arity!(impl_combination_tuple);

impl<T> sealed::Sealed for Vec<T> {}

impl<T> Combination for Vec<T> {
    type Latest = Vec<Option<T>>;
    type Queues = Vec<VecDeque<T>>;
    type After<U> = (U, Vec<T>);
}

impl<T> PrependOps for Vec<T> {
    fn prepend<U>(self, first: U) -> Self::After<U> {
        (first, self)
    }
}

impl<T> LatestOps for Vec<T> {
    fn empty(position_count: usize) -> Self::Latest {
        std::iter::repeat_with(|| None)
            .take(position_count)
            .collect()
    }

    fn take_all(storage: &mut Self::Latest) -> Self {
        storage
            .iter_mut()
            .map(|item| item.take().expect("every position has a value"))
            .collect()
    }
}

impl<T: Clone> CloneLatest for Vec<T> {
    fn clone_all(storage: &Self::Latest) -> Option<Self> {
        storage.iter().all(Option::is_some).then(|| {
            storage
                .iter()
                .map(|item| item.as_ref().expect("every position has a value").clone())
                .collect()
        })
    }
}

impl<T> QueueOps for Vec<T> {
    fn empty(position_count: usize) -> Self::Queues {
        std::iter::repeat_with(VecDeque::new)
            .take(position_count)
            .collect()
    }

    fn pop_all(storage: &mut Self::Queues) -> (Self, usize) {
        let mut empty = 0;
        let items = storage
            .iter_mut()
            .map(|queue| {
                let item = queue
                    .pop_front()
                    .expect("every position has a queued value");
                empty += usize::from(queue.is_empty());
                item
            })
            .collect();
        (items, empty)
    }
}

#[cfg(test)]
mod tests {
    use super::{CloneLatest, LatestOps, PrependOps, QueueOps};
    use crate::{
        thread_mode::mutable::MutableHelper,
        utils::positional::position::{AtPosition, TupleIndex},
    };
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
    };

    #[test]
    fn test_latest_tuple() {
        type Items = (i32, &'static str);
        let mut latest = <Items as LatestOps>::empty(2);
        assert_eq!(latest, (None, None));
        *latest.at_mut(TupleIndex::<0>) = Some(1);
        *latest.at_mut(TupleIndex::<1>) = Some("b");
        assert_eq!(Items::clone_all(&latest), Some((1, "b")));
        assert_eq!((1, "b").prepend('a'), ('a', 1, "b"));
        assert_eq!(Items::take_all(&mut latest), (1, "b"));
        assert_eq!(latest, (None, None));
    }

    #[test]
    fn test_queue_tuple() {
        type Items = (i32, char);
        let mut queues = <Items as QueueOps>::empty(2);
        assert_eq!(queues, (VecDeque::new(), VecDeque::new()));
        queues.at_mut(TupleIndex::<0>).extend([1, 2]);
        queues.at_mut(TupleIndex::<1>).push_back('a');
        assert_eq!(Items::pop_all(&mut queues), ((1, 'a'), 1));
        queues.at_mut(TupleIndex::<1>).push_back('b');
        assert_eq!(Items::pop_all(&mut queues), ((2, 'b'), 2));
    }

    #[test]
    fn test_latest_list() {
        type Items = Vec<i32>;
        let mut latest = <Items as LatestOps>::empty(3);
        assert_eq!(latest, [None, None, None]);
        for index in 0..3 {
            *latest.at_mut(index) = Some(index as i32);
        }
        assert_eq!(Items::clone_all(&latest), Some(vec![0, 1, 2]));
        assert_eq!(vec![0, 1, 2].prepend('a'), ('a', vec![0, 1, 2]));
        assert_eq!(Items::take_all(&mut latest), [0, 1, 2]);
        // The positions stay, without values.
        assert_eq!(latest, [None, None, None]);
    }

    #[test]
    fn test_queue_list() {
        type Items = Vec<i32>;
        let mut queues = <Items as QueueOps>::empty(12);
        assert_eq!(queues.len(), 12);
        assert!(queues.iter().all(VecDeque::is_empty));
        for index in 0..12 {
            queues.at_mut(index).push_back(index as i32);
        }
        queues.at_mut(7).push_back(-1);
        assert_eq!(
            Items::pop_all(&mut queues),
            ((0..12).collect::<Vec<_>>(), 11)
        );
        assert_eq!(queues.at(7), &[-1]);
    }

    #[test]
    fn test_clone_all_when_every_position_has_a_value() {
        type Items = (i32, &'static str);
        let mut latest = <Items as LatestOps>::empty(2);
        assert_eq!(Items::clone_all(&latest), None);
        *latest.at_mut(TupleIndex::<0>) = Some(1);
        assert_eq!(Items::clone_all(&latest), None);
        *latest.at_mut(TupleIndex::<1>) = Some("a");
        assert_eq!(Items::clone_all(&latest), Some((1, "a")));
        *latest.at_mut(TupleIndex::<0>) = Some(2);
        assert_eq!(Items::clone_all(&latest), Some((2, "a")));

        type List = Vec<i32>;
        let mut latest = <List as LatestOps>::empty(2);
        assert_eq!(List::clone_all(&latest), None);
        *latest.at_mut(1) = Some(1);
        assert_eq!(List::clone_all(&latest), None);
        *latest.at_mut(0) = Some(0);
        assert_eq!(List::clone_all(&latest), Some(vec![0, 1]));
    }

    #[test]
    fn test_clone_all_clones_nothing_until_complete() {
        struct Probe<'a>(&'a Cell<usize>);

        impl Clone for Probe<'_> {
            fn clone(&self) -> Self {
                self.0.set(self.0.get() + 1);
                Self(self.0)
            }
        }

        // A clone made before a missing position is found would be dropped under the lock.
        let clones = Cell::new(0);
        let mut latest = <(Probe<'_>, i32) as LatestOps>::empty(2);
        *latest.at_mut(TupleIndex::<0>) = Some(Probe(&clones));
        assert!(<(Probe<'_>, i32)>::clone_all(&latest).is_none());
        let mut list = <Vec<Probe<'_>> as LatestOps>::empty(2);
        *list.at_mut(0) = Some(Probe(&clones));
        assert!(<Vec<Probe<'_>>>::clone_all(&list).is_none());
        assert_eq!(clones.get(), 0);

        *latest.at_mut(TupleIndex::<1>) = Some(1);
        assert!(<(Probe<'_>, i32)>::clone_all(&latest).is_some());
        assert_eq!(clones.get(), 1);
    }

    #[test]
    fn test_empty_list() {
        // No position is ever missing.
        let latest = <Vec<i32> as LatestOps>::empty(0);
        assert_eq!(Vec::clone_all(&latest), Some(vec![]));
    }

    #[test]
    fn test_values_are_returned_without_dropping() {
        struct Probe<'a>(&'a RefCell<Vec<i32>>, i32);

        impl Drop for Probe<'_> {
            fn drop(&mut self) {
                self.0.with_mut(|drops| drops.push(self.1));
            }
        }

        let drops = RefCell::new(Vec::new());
        let mut latest = <Vec<Probe<'_>> as LatestOps>::empty(1);
        assert!(latest.at_mut(0).replace(Probe(&drops, 1)).is_none());
        let replaced = latest.at_mut(0).replace(Probe(&drops, 2));
        drops.with_ref(|drops| assert!(drops.is_empty()));
        drop(replaced);
        drops.with_ref(|drops| assert_eq!(*drops, [1]));

        drop(latest);
        drops.with_ref(|drops| assert_eq!(*drops, [1, 2]));
    }
}
