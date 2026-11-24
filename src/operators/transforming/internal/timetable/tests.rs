use super::{Timetable, Transition::*};
use crate::scheduler::{Clock, Next, SystemClock};
use std::time::Duration;

const MS: Duration = Duration::from_millis(1);

#[test]
fn test_overlapping() {
    let start = SystemClock.now();
    let mut timetable = Timetable::starting_at(start, 3 * MS, MS);
    assert_eq!(timetable.step(start), (None, Next::SleepUntil(start + MS)));
    assert_eq!(
        timetable.step(start + MS),
        (Some(Open), Next::SleepUntil(start + 2 * MS))
    );
    assert_eq!(timetable.step(start + 2 * MS).0, Some(Open));
    // The first closes as the fourth opens.
    assert_eq!(
        timetable.step(start + 3 * MS),
        (Some(Rotate), Next::SleepUntil(start + 4 * MS))
    );
}

#[test]
fn test_with_gaps() {
    let start = SystemClock.now();
    let mut timetable = Timetable::starting_at(start, MS, 3 * MS);
    assert_eq!(
        timetable.step(start + MS),
        (Some(Close), Next::SleepUntil(start + 3 * MS))
    );
    assert_eq!(
        timetable.step(start + 3 * MS),
        (Some(Open), Next::SleepUntil(start + 4 * MS))
    );
}

#[test]
fn test_late_one_instant_at_a_time() {
    let start = SystemClock.now();
    let mut timetable = Timetable::starting_at(start, 2 * MS, 2 * MS);
    assert_eq!(
        timetable.step(start + 5 * MS),
        (Some(Rotate), Next::SleepUntil(start + 4 * MS))
    );
    assert_eq!(
        timetable.step(start + 5 * MS),
        (Some(Rotate), Next::SleepUntil(start + 6 * MS))
    );
    assert_eq!(
        timetable.step(start + 5 * MS),
        (None, Next::SleepUntil(start + 6 * MS))
    );
}

#[test]
fn test_too_far_never_comes() {
    let start = SystemClock.now();
    // Nothing ever comes: the timer finishes at its first step.
    let mut timetable = Timetable::starting_at(start, Duration::MAX, Duration::MAX);
    assert_eq!(timetable.step(start), (None, Next::Finished));

    // Opens keep coming, but none of them ever closes.
    let mut timetable = Timetable::starting_at(start, Duration::MAX, MS);
    assert_eq!(
        timetable.step(start + MS),
        (Some(Open), Next::SleepUntil(start + 2 * MS))
    );
}

#[test]
fn test_finishes_after_last_representable_deadline() {
    let start = SystemClock.now();
    let mut timetable = Timetable::starting_at(start, MS, Duration::MAX);
    assert_eq!(timetable.step(start + MS), (Some(Close), Next::Finished));
}
