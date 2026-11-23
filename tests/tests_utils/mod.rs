#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod clone_probe;
pub(crate) mod drop_probe;
#[cfg(panic = "unwind")]
pub(crate) mod panic;
pub(crate) mod parked_observer;
pub(crate) mod scheduler_ext;
pub(crate) mod shared_sender;
pub(crate) mod test_channel;
pub(crate) mod test_scheduler;
pub(crate) mod test_struct;
pub(crate) mod thread_checker_scheduler;
pub(crate) mod window_checker;

use std::{
    fmt::Debug,
    ops::Sub,
    time::{Duration, Instant},
};

pub(crate) const DURATION_1_MS: Duration = Duration::from_millis(1);
pub(crate) const DURATION_3_MS: Duration = Duration::from_millis(3);
pub(crate) const DURATION_10_MS: Duration = Duration::from_millis(10);
pub(crate) const DURATION_30_MS: Duration = Duration::from_millis(30);
pub(crate) const DURATION_100_MS: Duration = Duration::from_millis(100);
/// A year: longer than any deadline a test means to reach, and still within the virtual clock.
pub(crate) const DURATION_1_YEAR: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// How long a test waits for another thread, through a channel, before it fails.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);

/// Checks values paired with a time read from a real clock, which is only close to the expected
/// one. On a virtual clock the times are exact: compare them with `assert_eq!`.
pub(crate) fn assert_times_within<T, C>(
    values: Vec<(T, C)>,
    expected: &[(T, C)],
    tolerance: Duration,
) where
    T: PartialEq + Debug,
    C: Ord + Copy + Debug + Sub<Output = Duration>,
{
    assert_eq!(values.len(), expected.len());
    for (value, expected) in values.iter().zip(expected) {
        assert_eq!(value.0, expected.0);
        let diff = value.1.max(expected.1) - value.1.min(expected.1);
        assert!(diff < tolerance, "{value:?} != {expected:?}");
    }
}

/// The longest duration that can be added to `from`: one nanosecond more is too far for an
/// [`Instant`] to represent. A deadline that long sits at the very end of the clock, so that the
/// same duration counted from any later time is out of range.
pub(crate) fn longest_duration_from(from: Instant) -> Duration {
    const NANOS_PER_SEC: u128 = 1_000_000_000;
    let duration = |nanos: u128| {
        Duration::new(
            (nanos / NANOS_PER_SEC) as u64,
            (nanos % NANOS_PER_SEC) as u32,
        )
    };
    let (mut low, mut high) = (0, Duration::MAX.as_nanos());
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if from.checked_add(duration(middle)).is_some() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    duration(low)
}
