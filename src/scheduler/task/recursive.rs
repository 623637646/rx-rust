use super::{Task, TaskState};
use std::{task::Poll, time::Instant};

/// The state of [`Task::recursive`].
pub struct RecursiveContext<C> {
    state: C,
    step: fn(&mut C, usize, Instant) -> TaskState,
    count: usize,
}

impl<C> Task<RecursiveContext<C>> {
    /// Calls `step(&mut state, count, now)` — `count` starting at 0, `now` the scheduler's time
    /// — until it returns [`TaskState::Finished`], sleeping or yielding in between as it asks.
    /// There is always a yield point between two steps, so that other tasks run and a disposal
    /// takes effect.
    pub fn recursive(state: C, step: fn(&mut C, usize, Instant) -> TaskState) -> Self {
        Self::new(
            RecursiveContext {
                state,
                step,
                count: 0,
            },
            |context, _, _, now| {
                let state = (context.step)(&mut context.state, context.count, now);
                context.count += 1;
                Poll::Ready(state)
            },
        )
    }
}
