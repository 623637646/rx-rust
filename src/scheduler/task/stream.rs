use super::{Next, Task};
use educe::Educe;
use futures::Stream;
use std::{ops::ControlFlow, task::Poll};

/// The state of [`Task::from_stream_then`]. `I` is the element type.
#[derive(Educe)]
#[educe(Debug)]
pub struct StreamThenContext<C, I> {
    #[educe(Debug(ignore))]
    state: Option<C>,
    on_item: fn(&mut C, I) -> ControlFlow<()>,
    on_end: fn(C),
}

impl<C, I, SM> Task<StreamThenContext<C, I>, SM>
where
    SM: Stream<Item = I>,
{
    /// Drives `stream` to its end: `on_item(&mut state, item)` for every element, then
    /// `on_end(state)`.
    ///
    /// If `on_item` returns `ControlFlow::Break(())`, the task drops the stream and `state` without
    /// polling the stream again or calling `on_end`. After `ControlFlow::Continue(())`, the task
    /// yields before polling again, even when the stream is always ready.
    ///
    /// Each element is taken, handled and dropped within one call of the handler, so
    /// `Task<StreamThenContext<C, I>, SM>: Send` exactly when `C: Send` and `SM: Send`.
    pub fn from_stream_then(
        state: C,
        stream: SM,
        on_item: fn(&mut C, I) -> ControlFlow<()>,
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
                        match (context.on_item)(state, item) {
                            ControlFlow::Continue(()) => Next::Yield,
                            ControlFlow::Break(()) => Next::Finished,
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
