use super::{Next, Task};

/// The state of [`Task::once`].
pub struct OnceContext<C> {
    state: Option<C>,
    run: fn(C),
}

impl<C> Task<OnceContext<C>> {
    /// Calls `run(state)` once.
    pub fn once(state: C, run: fn(C)) -> Self {
        Self::new(
            OnceContext {
                state: Some(state),
                run,
            },
            |context, _, _, _| {
                if let Some(state) = context.state.take() {
                    (context.run)(state);
                }
                Next::Finished
            },
        )
    }
}
