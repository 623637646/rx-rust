//! What the unit tests of the states share.

use super::{OnItem, SpanState};
use std::time::{Duration, Instant};

pub(in crate::operators::filtering) const DURATION_1_MS: Duration = Duration::from_millis(1);
pub(in crate::operators::filtering) const DURATION_100_MS: Duration = Duration::from_millis(100);

/// An item at `now`.
pub(in crate::operators::filtering) fn item<T>(
    state: &mut impl SpanState<T>,
    value: T,
    now: Instant,
    time_span: Duration,
) -> OnItem<T> {
    state.on_item(value, move || now, time_span)
}

/// An item that must not read the clock.
pub(in crate::operators::filtering) fn item_without_clock<T>(
    state: &mut impl SpanState<T>,
    value: T,
    time_span: Duration,
) -> OnItem<T> {
    state.on_item(
        value,
        || -> Instant { panic!("the item read the clock") },
        time_span,
    )
}
