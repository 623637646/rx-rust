//! The operators, one type per file, in the categories of
//! <https://reactivex.io/documentation/operators.html>.
//!
//! Sources are the types of [`creating`]; every other operator wraps a source and is normally
//! reached through the method of the same name on
//! [`ObservableExt`](crate::observable::ObservableExt). Each type's documentation has an example.
//!
//! | Module | Operators |
//! |---|---|
//! | [`creating`] | `Create`, `Defer`, `Empty`, `FromFuture`, `FromTryFuture`, `FromIter`, `FromResult`, `FromStream`, `FromTryStream`, `FromStreamAcked`, `FromTryStreamAcked`, `Interval`, `Just`, `Never`, `Range`, `Repeat`, `Start`, `Throw`, `Timer` |
//! | [`transforming`] | `map`, `scan`, `pairwise`, `flat_map`, `concat_map`, `concat_eager_map`, `switch_map`, `exhaust_map`, `group_by`, `buffer`, `buffer_with_count`, `buffer_with_count_and_skip`, `buffer_with_time`, `buffer_with_time_and_shift`, `buffer_with_time_or_count`, `window`, `window_with_count`, `window_with_count_and_skip`, `window_with_time`, `window_with_time_and_shift`, `window_with_time_or_count` |
//! | [`filtering`] | `filter`, `filter_map`, `first`, `last`, `single`, `element_at`, `take`, `take_last`, `skip`, `skip_last`, `distinct`, `distinct_with_key_selector`, `distinct_until_changed`, `distinct_until_changed_with_key_selector`, `debounce`, `audit`, `throttle`, `throttle_latest`, `sample`, `ignore_elements` |
//! | [`combining`] | `merge`, `merge_all`, `concat`, `concat_all`, `concat_eager`, `concat_eager_all`, `switch`, `exhaust_all`, `zip`, `combine_latest`, `with_latest_from`, `fork_join`, `join`, `group_join`, [`When`](combining::when::When), `start_with` |
//! | [`conditional_boolean`] | `all`, `contains`, `sequence_equal`, `default_if_empty`, `switch_if_empty`, `amb`, `take_while`, `take_until`, `skip_while`, `skip_until` |
//! | [`mathematical_aggregate`] | `count`, `sum`, `average`, `min`, `max`, `reduce`, `collect`, `to_vec` |
//! | [`error_handling`] | `catch`, `map_err`, `retry` |
//! | [`utility`] | `delay`, `delay_subscription`, `timeout`, `timestamp`, `time_interval`, `hold`, `do_finally`, `repeat_source`, `stop_on_disposal`, `materialize`, `dematerialize`, `subscribe_on`, `observe_on`, [`Using`](utility::using::Using), `do_before_*`, `do_after_*` |
//! | [`connectable`] | `multicast`, `publish`, `publish_behavior`, `publish_last`, `replay`, `ref_count`, `share`, `share_behavior`, `share_last`, `share_replay` |
//! | [`others`] | `into_shared`, `with_item_type`, `with_error_type`, `debug`, `debug_to_stdout`, `hook_on_*` |

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
