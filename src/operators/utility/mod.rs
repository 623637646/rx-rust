//! Operators that time, schedule, observe or wrap a sequence without changing its items.
//! See <https://reactivex.io/documentation/operators.html#utility>.
//!
//! `Using` is not implemented: a resource tied to a subscription is dropped with it, so an owned
//! value in the pipeline does the same. `Serialize` is not needed, since every operator here is
//! already safe to drive from several threads.

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
pub mod materialize;
pub mod observe_on;
pub mod subscribe_on;
pub mod time_interval;
pub mod timeout;
pub mod timestamp;
