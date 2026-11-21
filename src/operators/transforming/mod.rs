//! Operators that transform items, or regroup them into buffers, windows and groups.
//! See <https://reactivex.io/documentation/operators.html#transforming>.

pub mod buffer;
pub mod buffer_with_count;
pub mod buffer_with_count_and_skip;
pub mod buffer_with_time;
pub mod buffer_with_time_and_shift;
pub mod buffer_with_time_or_count;
pub mod concat_eager_map;
pub mod concat_map;
pub mod exhaust_map;
pub mod flat_map;
pub mod group_by;
mod internal;
pub mod map;
pub mod pairwise;
pub mod scan;
pub mod switch_map;
pub mod window;
pub mod window_with_count;
pub mod window_with_count_and_skip;
pub mod window_with_time;
pub mod window_with_time_and_shift;
pub mod window_with_time_or_count;
