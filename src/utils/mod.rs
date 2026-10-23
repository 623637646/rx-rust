//! The machinery the operators are built from.
//!
//! The public modules are for anyone writing an operator of their own:
//!
//! - [`subscribe_with_context`]: the helper most stateful operators are written with — one lock
//!   around a model and the downstream observer, with events delivered outside it.
//! - [`subscribe_with_auto_dispose_on_termination`]: the helper for an operator that ends the
//!   stream early, disposing its source when it does.
//! - [`serialized_delivery`]: the serialized delivery to one observer that the first helper is
//!   made of. It takes its events as an [`EventBatch`](crate::observer::EventBatch).
//!
//! The locks, and the only sanctioned way to reach through them, live with the
//! [thread mode](crate::thread_mode) that picks them, in
//! [`thread_mode::mutable`](crate::thread_mode::mutable).
//!
//! The other modules are private to the crate: the serialized delivery to many observers that the
//! subjects are made of, the re-subscription of an observer that subscribes an observable with an
//! observer of its own type (as `retry` and `concat_all` do), the queue behind a running delivery,
//! the reservation slot, the id generator, the panic guard and the lazy subscription of the
//! future and stream adapters. They serve the crate's own operators and appear in no public
//! signature.
//!
//! Nothing here is needed to *use* the operators; see the module documentation of each part for
//! the rules it enforces.

pub(crate) mod id_generator;
pub(crate) mod lazy_subscription;
pub(crate) mod on_panic;
pub(crate) mod pending_events;
pub(crate) mod reservation_slot;
pub(crate) mod resubscribe;
pub mod serialized_delivery;
pub(crate) mod serialized_multicast;
pub mod subscribe_with_auto_dispose_on_termination;
pub mod subscribe_with_context;

use std::marker::PhantomData;

/// Placeholder for a type parameter that a struct carries but never stores.
///
/// `PhantomData<fn(T) -> T>` rather than `PhantomData<T>`, so the marker is `Send` / `Sync`
/// whatever `T` is, and invariant in `T`
/// (<https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>).
///
/// Operators carry one for a type they name but do not store: a type the caller picks, such as the
/// item type of `with_item_type`, or the item type of their source, which keeps the operator's type
/// spelling the types it converts between. The marker ties the struct's lifetime to `T` —
/// `MarkerType<&'a U>` is not `'static`, and no spelling of `PhantomData` avoids that — but the
/// source observable already carries the same lifetime, so it costs nothing.
pub(crate) type MarkerType<T> = PhantomData<fn(T) -> T>;

/// Takes the values out of `values`, a buffer about to be filled again, leaving it with room for
/// `room` of them; or, while it is empty, a new `Vec`, which allocates nothing and leaves the buffer
/// the room it has.
///
/// `room` is the caller's guess at the next batch: the length of this one, which follows the pace
/// of the source and settles back after a burst, or a known bound such as a bundle's `count`.
pub(crate) fn take_leaving_room<T>(values: &mut Vec<T>, room: usize) -> Vec<T> {
    if values.is_empty() {
        return Vec::new();
    }
    std::mem::replace(values, Vec::with_capacity(room))
}

#[cfg(test)]
mod tests {
    use super::take_leaving_room;

    #[test]
    fn test_take_leaving_room() {
        let mut values = Vec::with_capacity(4);
        values.extend([1, 2, 3]);

        let taken = take_leaving_room(&mut values, 8);
        assert_eq!(taken, [1, 2, 3]);
        assert!(values.is_empty());
        assert!(values.capacity() >= 8);
    }

    #[test]
    fn test_take_leaving_room_empty() {
        let mut values = Vec::<i32>::with_capacity(4);

        // Nothing to take: nothing is allocated, and the buffer keeps its room.
        let taken = take_leaving_room(&mut values, 8);
        assert!(taken.is_empty());
        assert_eq!(taken.capacity(), 0);
        assert!(values.capacity() >= 4);
    }
}
