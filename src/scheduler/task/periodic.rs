use super::{Task, TaskState};
use std::{
    task::Poll,
    time::{Duration, Instant},
};

/// The state of [`Task::periodic`].
pub struct PeriodicContext<C> {
    state: C,
    step: fn(&mut C, usize) -> bool,
    period: Duration,
    /// When the next step is due. `None` until the first step, whose time then anchors the rate.
    next_time: Option<Instant>,
    count: usize,
}

impl<C> Task<PeriodicContext<C>> {
    /// Calls `step(&mut state, count)` — `count` starting at 0 — at a fixed rate, until it returns
    /// `false`.
    ///
    /// The n-th step is due at `anchor + n * period`; without an anchor, the first step's time is
    /// the anchor. A step that overruns the period is followed by the missed ones back to back,
    /// never skipped, each after a yield point.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    pub fn periodic(
        state: C,
        step: fn(&mut C, usize) -> bool,
        period: Duration,
        anchor: Option<Instant>,
    ) -> Self {
        assert!(!period.is_zero(), "period must be non-zero");
        Self::new(
            PeriodicContext {
                state,
                step,
                period,
                next_time: anchor,
                count: 0,
            },
            |context, _, _| {
                let next_time = context.next_time.get_or_insert_with(Instant::now);
                if !(context.step)(&mut context.state, context.count) {
                    return Poll::Ready(TaskState::Finished);
                }
                context.count += 1;
                *next_time += context.period;
                Poll::Ready(TaskState::SleepUntil(*next_time))
            },
        )
    }
}
