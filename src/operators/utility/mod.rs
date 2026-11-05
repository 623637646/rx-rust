//! Operators that time, schedule, observe or wrap a sequence without changing its items.
//! See <https://reactivex.io/documentation/operators.html#utility>.
//!
//! A resource tied to a subscription is an owned value, dropped with the observer that holds it,
//! so neither [`Using`](using::Using) nor [`hold`](crate::observable::ObservableExt::hold) takes a
//! function to release it, and neither releases it through its disposal. `Using` creates the
//! resource for each subscription and drops it before the termination goes downstream; `hold`
//! attaches a value that already exists, and drops it after.
//! `Serialize` is not needed: `on_next` takes `&mut self`, so the type system already serializes
//! the calls into an observer.

pub mod delay;
pub mod dematerialize;
pub mod do_after_disposal;
pub mod do_after_next;
pub mod do_after_subscription;
pub mod do_after_termination;
pub mod do_before_disposal;
pub mod do_before_next;
pub mod do_before_subscription;
pub mod do_before_termination;
pub mod hold;
pub mod materialize;
pub mod observe_on;
pub mod subscribe_on;
pub mod time_interval;
pub mod timeout;
pub mod timestamp;
pub mod using;
