//! When the bundles or windows of a time-and-shift operator close and the next one opens, shared
//! by [`BufferWithTimeAndShift`](super::super::buffer_with_time_and_shift::BufferWithTimeAndShift)
//! and [`WindowWithTimeAndShift`](super::super::window_with_time_and_shift::WindowWithTimeAndShift)
//! (decision 0014).
//!
//! Each group lasts `time_span`, so openings and closings both recur every `time_shift`. Closings
//! follow opening order, as the [`Overlap`](super::overlap::Overlap) expects. Deadlines stay
//! anchored to the start even when the timer runs late. A deadline outside the range of
//! [`Instant`] ends its sequence.

use crate::scheduler::Next;
use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

/// The next closing and opening deadlines. Both recur every `time_shift`.
pub(crate) struct Timetable {
    next_close: Option<Instant>,
    next_open: Option<Instant>,
    time_shift: Duration,
}

/// What is due at one instant.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Transition {
    /// Close the oldest.
    Close,
    /// Open a new one.
    Open,
    /// Close the oldest, then open a new one.
    Rotate,
}

impl Timetable {
    /// The timetable of the first one, opened at `start`, the caller's to open.
    ///
    /// # Panics
    ///
    /// Panics if `time_span` or `time_shift` is zero.
    pub(crate) fn starting_at(start: Instant, time_span: Duration, time_shift: Duration) -> Self {
        assert!(!time_span.is_zero(), "time_span must be non-zero");
        assert!(!time_shift.is_zero(), "time_shift must be non-zero");
        Self {
            next_close: start.checked_add(time_span),
            next_open: start.checked_add(time_shift),
            time_shift,
        }
    }

    /// The earliest deadline and what is due at it, `None` when both are out of range.
    fn next_transition(&self) -> Option<(Instant, Transition)> {
        match (self.next_close, self.next_open) {
            (Some(close), Some(open)) => Some(match close.cmp(&open) {
                Ordering::Less => (close, Transition::Close),
                Ordering::Equal => (close, Transition::Rotate),
                Ordering::Greater => (open, Transition::Open),
            }),
            (Some(close), None) => Some((close, Transition::Close)),
            (None, Some(open)) => Some((open, Transition::Open)),
            (None, None) => None,
        }
    }

    /// Takes at most one due transition. Any later transition gets a separate step, including
    /// when the timer is catching up.
    pub(crate) fn step(&mut self, now: Instant) -> (Option<Transition>, Next) {
        let Some((at, transition)) = self.next_transition() else {
            return (None, Next::Finished);
        };
        if at > now {
            return (None, Next::SleepUntil(at));
        }

        let following = at.checked_add(self.time_shift);
        match transition {
            Transition::Close => self.next_close = following,
            Transition::Open => self.next_open = following,
            Transition::Rotate => {
                self.next_close = following;
                self.next_open = following;
            }
        }
        let next = self
            .next_transition()
            .map_or(Next::Finished, |(at, _)| Next::SleepUntil(at));
        (Some(transition), next)
    }
}

#[cfg(test)]
mod tests {
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
}
