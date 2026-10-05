use super::{Task, TaskState};
use std::task::Poll;

/// The state of [`Task::recursive`].
pub struct RecursiveContext<C> {
    state: C,
    step: fn(&mut C, usize) -> TaskState,
    count: usize,
}

impl<C> Task<RecursiveContext<C>> {
    /// Calls `step(&mut state, count)` — `count` starting at 0 — until it returns
    /// [`TaskState::Finished`], sleeping or yielding in between as it asks. There is always a yield
    /// point between two steps, so that other tasks run and a disposal takes effect.
    pub fn recursive(state: C, step: fn(&mut C, usize) -> TaskState) -> Self {
        Self::new(
            RecursiveContext {
                state,
                step,
                count: 0,
            },
            |context, _, _| {
                let state = (context.step)(&mut context.state, context.count);
                context.count += 1;
                Poll::Ready(state)
            },
        )
    }
}
