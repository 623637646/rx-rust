//! The machinery the operators are built from.
//!
//! The public modules are for anyone writing an operator of their own:
//!
//! - [`subscribe_with_context`]: the helper most stateful operators are written with — one lock
//!   around a model and the downstream observer, with events delivered outside it.
//! - [`subscribe_with_auto_dispose_on_termination`]: the helper for an operator that ends the
//!   stream early, disposing its source when it does.
//! - [`serialized_delivery`], [`serialized_multicast`]: the serialized delivery to one observer
//!   that the first helper is made of, and the one to many observers that the subjects are made
//!   of. Both take their events as an [`EventBatch`](crate::observer::EventBatch).
//! - [`resubscribe`]: for an operator whose observer subscribes an observable with an observer of
//!   its own type, as `retry` and `concat_all` do.
//!
//! The locks, and the only sanctioned way to reach through them, live with the
//! [thread mode](crate::thread_mode) that picks them, in
//! [`thread_mode::mutable`](crate::thread_mode::mutable).
//!
//! The other modules are private to the crate: the queue behind a running delivery, the slot of a
//! single inner subscription, the id generator, the panic guard and the lazy subscription of the
//! future and stream adapters. They serve the crate's own operators and appear in no public
//! signature.
//!
//! Nothing here is needed to *use* the operators; see the module documentation of each part for
//! the rules it enforces.

pub(crate) mod id_generator;
pub(crate) mod lazy_subscription;
pub(crate) mod on_panic;
pub(crate) mod pending_events;
pub mod resubscribe;
pub mod serialized_delivery;
pub mod serialized_multicast;
pub mod subscribe_with_auto_dispose_on_termination;
pub mod subscribe_with_context;
pub(crate) mod subscription_slot;

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
