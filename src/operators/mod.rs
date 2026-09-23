//! The operators, one type per file, in the categories of <https://reactivex.io/documentation/operators.html>.
//!
//! Sources are the types of [`creating`]; every other operator wraps a source and is normally
//! reached through the method of the same name on
//! [`ObservableExt`](crate::observable::ObservableExt). Each type's documentation has an example.
//!
//! | Module | Operators |
//! |---|---|
//! | [`creating`] | `Create`, `Defer`, `Empty`, `FromFuture`, `FromTryFuture`, `FromIter`, `FromResult`, `FromStream`, `FromTryStream`, `Interval`, `Just`, `Never`, `Range`, `Repeat`, `Start`, `Throw`, `Timer` |
//! | [`transforming`] | `map`, `scan`, `flat_map`, `concat_map`, `switch_map`, `group_by`, `buffer`, `buffer_with_count`, `buffer_with_time`, `buffer_with_time_or_count`, `window`, `window_with_count` |
//! | [`filtering`] | `filter`, `first`, `last`, `element_at`, `take`, `take_last`, `skip`, `skip_last`, `distinct`, `distinct_until_changed`, `debounce`, `throttle`, `sample`, `ignore_elements` |
//! | [`combining`] | `merge_with`, `merge_all`, `concat_with`, `concat_all`, `switch`, `zip`, `combine_latest`, `start_with` |
//! | [`conditional_boolean`] | `all`, `contains`, `sequence_equal`, `default_if_empty`, `amb_with`, `take_while`, `take_until`, `skip_while`, `skip_until` |
//! | [`mathematical_aggregate`] | `count`, `sum`, `average`, `min`, `max`, `reduce`, `collect`, `to_vec` |
//! | [`error_handling`] | `catch`, `map_err`, `retry` |
//! | [`utility`] | `delay`, `timeout`, `timestamp`, `time_interval`, `materialize`, `dematerialize`, `subscribe_on`, `observe_on`, `do_before_*`, `do_after_*` |
//! | [`connectable`] | `multicast`, `publish`, `publish_last`, `replay`, `share`, `share_last`, `share_replay` |
//! | [`others`] | `into_future`, `into_try_future`, `into_stream`, `into_try_stream`, `with_item_type`, `with_error_type`, `debug`, `hook_on_*` |

pub mod combining;
pub mod conditional_boolean;
pub mod connectable;
pub mod creating;
pub mod error_handling;
pub mod filtering;
pub mod mathematical_aggregate;
pub mod others;
pub mod transforming;
pub mod utility;
