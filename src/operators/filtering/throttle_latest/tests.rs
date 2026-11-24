use super::ThrottleLatestState;
use crate::{
    operators::filtering::internal::span::{
        OnItem, OnTimer, SpanState,
        testing::{DURATION_1_MS, DURATION_100_MS, item, item_without_clock},
    },
    scheduler::virtual_time::VirtualTime,
};
use std::time::Duration;

const HELD: OnItem<i32> = OnItem::Hold {
    wake_timer: true,
    replaced: None,
};

#[test]
fn test_emits_at_once_then_the_last_item_of_each_span() {
    let start = VirtualTime::new().now();
    let end = start + DURATION_100_MS;
    let mut state = ThrottleLatestState::default();

    assert_eq!(item(&mut state, 1, start, DURATION_100_MS), OnItem::Emit(1));
    // The span holds nothing: the timer does not wait for it.
    assert_eq!(state.on_timer(start, DURATION_100_MS), OnTimer::WaitForItem);
    assert_eq!(
        item(&mut state, 2, start + DURATION_1_MS, DURATION_100_MS),
        HELD
    );
    // Keeps the end of the span, so it does not read the clock.
    assert_eq!(
        item_without_clock(&mut state, 3, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: false,
            replaced: Some(2)
        }
    );
    assert_eq!(
        state.on_timer(end - DURATION_1_MS, DURATION_100_MS),
        OnTimer::WaitUntil(end)
    );
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::Emit(3));

    // The emission opened the next span, until one time span later.
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::WaitForItem);
    assert_eq!(
        item(
            &mut state,
            4,
            end + DURATION_100_MS - DURATION_1_MS,
            DURATION_100_MS
        ),
        HELD
    );
    assert_eq!(state.on_completed(), Some(4));
}

/// No timer step ends a span that holds nothing: the next item finds it ended, from its end
/// on, and is emitted at once.
#[test]
fn test_span_holding_nothing_ends_without_the_timer() {
    let start = VirtualTime::new().now();
    let end = start + DURATION_100_MS;
    let mut state = ThrottleLatestState::default();

    assert_eq!(item(&mut state, 1, start, DURATION_100_MS), OnItem::Emit(1));
    assert_eq!(item(&mut state, 2, end, DURATION_100_MS), OnItem::Emit(2));
    // That item opened a span of its own.
    assert_eq!(
        item(
            &mut state,
            3,
            end + DURATION_100_MS - DURATION_1_MS,
            DURATION_100_MS
        ),
        HELD
    );
}

/// A span that ended holding a value is the timer's to close: until its step, an item replaces
/// that value, which the step then emits.
#[test]
fn test_late_timer_emits_the_last_item() {
    let start = VirtualTime::new().now();
    let end = start + DURATION_100_MS;
    let mut state = ThrottleLatestState::default();

    assert_eq!(item(&mut state, 1, start, DURATION_100_MS), OnItem::Emit(1));
    assert_eq!(
        item(&mut state, 2, start + DURATION_1_MS, DURATION_100_MS),
        HELD
    );
    assert_eq!(
        item_without_clock(&mut state, 3, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: false,
            replaced: Some(2)
        }
    );
    assert_eq!(
        state.on_timer(end + DURATION_1_MS, DURATION_100_MS),
        OnTimer::Emit(3)
    );
}

#[test]
fn test_time_span_too_long_waits_for_the_completion() {
    let start = VirtualTime::new().now();
    let mut state = ThrottleLatestState::default();

    assert_eq!(item(&mut state, 1, start, Duration::MAX), OnItem::Emit(1));
    assert_eq!(
        item(&mut state, 2, start + DURATION_1_MS, Duration::MAX),
        OnItem::Hold {
            wake_timer: false,
            replaced: None
        }
    );
    assert_eq!(state.on_timer(start, Duration::MAX), OnTimer::WaitForItem);
    assert_eq!(state.on_completed(), Some(2));
}
