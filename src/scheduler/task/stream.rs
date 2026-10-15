use super::{Next, Task};
use futures::Stream;
use std::task::Poll;

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
                let Poll::Ready(item) = stream.poll_next(cx) else {
                    return Next::Pending;
                };
                match item {
                    Some(item) => {
                        let state = context
                            .state
                            .as_mut()
                            .expect("from_stream_then handler called after completion");
                        if (context.on_item)(state, item) {
                            Next::Yield
                        } else {
                            Next::Finished
                        }
                    }
                    None => {
                        let state = context
                            .state
                            .take()
                            .expect("from_stream_then handler called after completion");
                        (context.on_end)(state);
                        Next::Finished
                    }
                }
            },
        )
    }
}
