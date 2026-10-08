use super::{Task, TaskState};
use futures::Stream;

/// The state of [`Task::from_stream_then`]. `I` is the element type.
pub struct StreamThenContext<C, I> {
    state: Option<C>,
    on_item: fn(&mut C, I) -> bool,
    on_end: fn(C),
}

impl<C, I, SM> Task<StreamThenContext<C, I>, SM>
where
    SM: Stream<Item = I>,
{
    /// Drives `stream` to its end: `on_item(&mut state, item)` for every element, then
    /// `on_end(state)`.
    ///
    /// `on_item` returning `false` stops right there: the stream is not polled again, `on_end` is
    /// not called, and the stream and `state` are dropped with the task. There is a yield point
    /// after every element, even when the stream is always ready.
    ///
    /// Each element is taken, handled and dropped within one call of the handler, so
    /// `Task<StreamThenContext<C, I>, SM>: Send` exactly when `C: Send` and `SM: Send`.
    pub fn from_stream_then(
        state: C,
        stream: SM,
        on_item: fn(&mut C, I) -> bool,
        on_end: fn(C),
    ) -> Self {
        Self::with_pinned(
            StreamThenContext {
                state: Some(state),
                on_item,
                on_end,
            },
            stream,
            |context, stream, cx, _| {
                stream.poll_next(cx).map(|item| match item {
                    Some(item) => {
                        let state = context
                            .state
                            .as_mut()
                            .expect("from_stream_then handler called after completion");
                        if (context.on_item)(state, item) {
                            TaskState::Yield
                        } else {
                            TaskState::Finished
                        }
                    }
                    None => {
                        let state = context
                            .state
                            .take()
                            .expect("from_stream_then handler called after completion");
                        (context.on_end)(state);
                        TaskState::Finished
                    }
                })
            },
        )
    }
}

impl<SM, F> Task<StreamThenContext<F, SM::Item>, SM>
where
    SM: Stream,
    F: FnMut(Option<SM::Item>) -> bool,
{
    /// Drives `stream` to its end: `callback(Some(item))` for every element, then
    /// `callback(None)`. `callback` returning `false` stops right there, and the final `None` is
    /// then never delivered.
    pub fn from_stream(stream: SM, callback: F) -> Self {
        Self::from_stream_then(
            callback,
            stream,
            |callback, item| callback(Some(item)),
            |mut callback| {
                callback(None);
            },
        )
    }
}
