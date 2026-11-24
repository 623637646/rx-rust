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
mod tests;
