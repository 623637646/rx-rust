//! The machinery the operators are built from, public for anyone writing an operator of their own.
//!
//! - [`types`] and [`mutable`]: the shared pointer, the lock and the `Send` bounds that switch
//!   between the single-threaded and the multi-threaded build.
//! - [`subscribe_with_context`]: the helper most stateful operators are written with — one lock
//!   around a model and the downstream observer, with events delivered outside it.
//! - [`subscribe_with_auto_dispose_on_termination`]: the helper for an operator that ends the
//!   stream early, disposing its source when it does.
//! - [`serialized_delivery`], [`serialized_multicast`], [`pending_events`], [`subscription_slot`],
//!   [`id_generator`], [`on_panic`]: what those two helpers, and the subjects, are made of.
//!
//! Nothing here is needed to *use* the operators; see the module documentation of each part for
//! the rules it enforces.

pub mod id_generator;
pub mod mutable;
pub mod on_panic;
pub mod pending_events;
pub mod serialized_delivery;
pub mod serialized_multicast;
pub mod subscribe_with_auto_dispose_on_termination;
pub mod subscribe_with_context;
pub mod subscription_slot;
pub mod types;
