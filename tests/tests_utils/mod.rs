#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod clone_probe;
pub(crate) mod drop_probe;
#[cfg(panic = "unwind")]
pub(crate) mod panic;
pub(crate) mod shared_sender;
pub(crate) mod test_channel;
pub(crate) mod test_scheduler;
pub(crate) mod test_struct;
pub(crate) mod thread_checker_scheduler;

use std::time::{Duration, Instant};

pub(crate) const DURATION_1_MS: Duration = Duration::from_millis(1);
pub(crate) const DURATION_3_MS: Duration = Duration::from_millis(3);
pub(crate) const DURATION_10_MS: Duration = Duration::from_millis(10);
pub(crate) const DURATION_30_MS: Duration = Duration::from_millis(30);
pub(crate) const DURATION_100_MS: Duration = Duration::from_millis(100);
/// A year: longer than any deadline a test means to reach, and still within the virtual clock.
pub(crate) const DURATION_1_YEAR: Duration = Duration::from_secs(365 * 24 * 60 * 60);

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
