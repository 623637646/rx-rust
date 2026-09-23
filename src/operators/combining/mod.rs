//! Operators that combine several observables into one, or flatten an observable of observables.
//! See <https://reactivex.io/documentation/operators.html#combining>.
//!
//! `And` / `Then` / `When` and `Join` are not implemented: `zip`, `combine_latest`, `flat_map`,
//! `take_until` and `window` cover their uses.

pub mod combine_latest;
pub mod concat;
pub mod concat_all;
pub mod merge;
pub mod merge_all;
pub mod start_with;
pub mod switch;
pub mod zip;
