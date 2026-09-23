//! Sources: observables that emit from a value, an iterator, a future, a stream, a timer, or a
//! closure. See <https://reactivex.io/documentation/operators.html#creating>.
//!
//! These are types, not methods: a pipeline starts with one of them, e.g.
//! [`FromIter::new(vec![1, 2, 3])`](from_iter::FromIter) or [`Just::new(1)`](just::Just).

pub mod create;
pub mod defer;
pub mod empty;
pub mod from_future;
pub mod from_iter;
pub mod from_result;
#[cfg(feature = "futures")]
pub mod from_stream;
pub mod from_try_future;
#[cfg(feature = "futures")]
pub mod from_try_stream;
pub mod interval;
pub mod just;
pub mod never;
pub mod range;
pub mod repeat;
pub mod start;
pub mod throw;
pub mod timer;
