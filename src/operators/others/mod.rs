//! Type and mode adapters, and debugging hooks.
//!
//! What an observable becomes as a future or a stream is not an operator, and lives with the
//! observable: [`observable::future`](crate::observable::future) and `observable::stream` (with
//! the `futures` feature).

pub mod debug;
pub mod hook_on_next;
pub mod hook_on_subscription;
pub mod hook_on_termination;
pub mod into_shared;
pub mod with_error_type;
pub mod with_item_type;
