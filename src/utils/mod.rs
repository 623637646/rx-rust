//! The machinery the operators are built from.
//!
//! The public modules are for anyone writing an operator of their own:
//!
//! - [`subscribe_with_context`]: the helper most stateful operators are written with — one lock
//!   around a model and the downstream observer, with events delivered outside it.
//! - [`subscribe_with_auto_dispose_on_termination`]: the helper for an operator that ends the
//!   stream early, disposing its source when it does.
//! - [`serialized_delivery`], [`serialized_multicast`], [`pending_events`], [`subscription_slot`],
//!   [`id_generator`], [`on_panic`]: what those two helpers, and the subjects, are made of.
//! - [`resubscribe`]: for an operator whose observer subscribes an observable with an observer of
//!   its own type, as `retry` and `concat_all` do.
//! - [`lazy_subscription`]: for an adapter that turns an observable into a future or a stream,
//!   subscribing on the first poll and releasing the source when the adapter is over.
//!
//! The locks, and the only sanctioned way to reach through them, live with the
//! [thread mode](crate::thread_mode) that picks them, in
//! [`thread_mode::mutable`](crate::thread_mode::mutable).
//!
//! Nothing here is needed to *use* the operators; see the module documentation of each part for
//! the rules it enforces.

pub mod id_generator;
pub mod lazy_subscription;
pub mod on_panic;
pub mod pending_events;
pub mod resubscribe;
pub mod serialized_delivery;
pub mod serialized_multicast;
pub mod subscribe_with_auto_dispose_on_termination;
pub mod subscribe_with_context;
pub mod subscription_slot;

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
