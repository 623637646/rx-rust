use super::DebounceState;
use crate::{
    operators::filtering::internal::span::{
        OnItem, OnTimer, SpanState,
        testing::{DURATION_1_MS, DURATION_100_MS, item},
    },
    scheduler::virtual_time::VirtualTime,
};
use std::time::Duration;

#[test]
fn test_each_item_moves_the_end() {
    let start = VirtualTime::new().now();
    let half = DURATION_100_MS / 2;
    let end = start + half + DURATION_100_MS;
    let mut state = DebounceState::default();

    assert_eq!(
        item(&mut state, 1, start, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: true,
            replaced: None
        }
    );
    assert_eq!(
        item(&mut state, 2, start + half, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: false,
            replaced: Some(1)
        }
    );
    // Woken at the first end, the timer waits on for the moved one.
    assert_eq!(
        state.on_timer(start + DURATION_100_MS, DURATION_100_MS),
        OnTimer::WaitUntil(end)
    );
    assert_eq!(
        state.on_timer(end - DURATION_1_MS, DURATION_100_MS),
        OnTimer::WaitUntil(end)
    );
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::Emit(2));
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::WaitForItem);
    assert_eq!(state.on_completed(), None);
}

#[test]
fn test_time_span_too_long_waits_for_the_completion() {
    let start = VirtualTime::new().now();
    let mut state = DebounceState::default();

    assert_eq!(
        item(&mut state, 1, start, Duration::MAX),
        OnItem::Hold {
            wake_timer: false,
            replaced: None
        }
    );
    assert_eq!(
        item(&mut state, 2, start + DURATION_1_MS, Duration::MAX),
        OnItem::Hold {
            wake_timer: false,
            replaced: Some(1)
        }
    );
    assert_eq!(state.on_timer(start, Duration::MAX), OnTimer::WaitForItem);
    assert_eq!(state.on_completed(), Some(2));
}
