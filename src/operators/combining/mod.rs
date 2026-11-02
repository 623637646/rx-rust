//! Operators that combine several observables into one, or flatten an observable of observables.
//! See <https://reactivex.io/documentation/operators.html#combining>.
//!
//! `And` / `Then` / `When` is not implemented: `zip`, `combine_latest`, `flat_map`, `take_until`
//! and `window` cover its uses.

pub mod combine_latest;
pub mod concat;
pub mod concat_all;
pub mod group_join;
pub mod join;
mod join_core;
pub mod merge;
pub mod merge_all;
pub mod start_with;
pub mod switch;
pub mod zip;
