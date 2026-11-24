use super::AuditState;
use crate::{
    operators::filtering::internal::span::{
        OnItem, OnTimer, SpanState,
        testing::{DURATION_1_MS, DURATION_100_MS, item, item_without_clock},
    },
    scheduler::virtual_time::VirtualTime,
};
use std::time::Duration;

#[test]
fn test_holds_the_last_item_until_the_span_ends() {
    let start = VirtualTime::new().now();
    let end = start + DURATION_100_MS;
    let mut state = AuditState::default();

    assert_eq!(
        item(&mut state, 1, start, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: true,
            replaced: None
        }
    );
    // Keeps the end of the span, so it does not read the clock.
    assert_eq!(
        item_without_clock(&mut state, 2, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: false,
            replaced: Some(1)
        }
    );
    assert_eq!(
        state.on_timer(end - DURATION_1_MS, DURATION_100_MS),
        OnTimer::WaitUntil(end)
    );
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::Emit(2));

    // The emission closed the span: the next item opens another.
    assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::WaitForItem);
    assert_eq!(
        item(&mut state, 3, end + DURATION_1_MS, DURATION_100_MS),
        OnItem::Hold {
            wake_timer: true,
            replaced: None
        }
    );
    assert_eq!(state.on_completed(), Some(3));
}

#[test]
fn test_time_span_too_long_waits_for_the_completion() {
    let start = VirtualTime::new().now();
    let mut state = AuditState::default();

    assert_eq!(
        item(&mut state, 1, start, Duration::MAX),
        OnItem::Hold {
            wake_timer: false,
            replaced: None
        }
    );
    assert_eq!(state.on_timer(start, Duration::MAX), OnTimer::WaitForItem);
    assert_eq!(state.on_completed(), Some(1));
    assert_eq!(state.on_completed(), None);
}
