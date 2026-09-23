//! Operators that select which items to pass on, by predicate, position, distinctness or timing.
//! See <https://reactivex.io/documentation/operators.html#filtering>.

pub mod debounce;
pub mod distinct;
pub mod distinct_until_changed;
pub mod element_at;
pub mod filter;
pub mod first;
pub mod ignore_elements;
pub mod last;
pub mod sample;
pub mod skip;
pub mod skip_last;
pub mod take;
pub mod take_last;
pub mod throttle;
