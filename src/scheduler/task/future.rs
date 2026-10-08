use super::{Task, TaskState};
use std::future::Future;

/// The state of [`Task::from_future_then`]. `O` is the future's output.
pub struct FutureThenContext<C, O> {
    state: Option<C>,
    on_ready: fn(C, O),
}

impl<C, O, FU> Task<FutureThenContext<C, O>, FU>
where
    FU: Future<Output = O>,
{
    /// Drives `future` to completion, then calls `on_ready(state, output)`.
    ///
    /// `Task<FutureThenContext<C, O>, FU>: Send` exactly when `C: Send` and `FU: Send`: the output
    /// is taken and consumed within one call of the handler, never across a yield point.
    pub fn from_future_then(state: C, future: FU, on_ready: fn(C, O)) -> Self {
        Self::with_pinned(
            FutureThenContext {
                state: Some(state),
                on_ready,
            },
            future,
            |context, future, cx, _| {
                future.poll(cx).map(|output| {
                    let state = context
                        .state
                        .take()
                        .expect("from_future_then handler called after completion");
                    (context.on_ready)(state, output);
                    TaskState::Finished
                })
            },
        )
    }
}

impl<FU> Task<FutureThenContext<(), ()>, FU>
where
    FU: Future<Output = ()>,
{
    /// Drives `future` to completion.
    pub fn from_future(future: FU) -> Self {
        Self::from_future_then((), future, |(), ()| {})
    }
}
