use super::{Next, Task};
use crate::thread_mode::{ThreadMode, mutable::MutableHelper};
use educe::Educe;
use futures::Stream;
use std::{
    num::NonZeroUsize,
    task::{Poll, Waker},
};

/// The state of [`Task::from_stream_acked_then`]. `I` is the element type.
///
/// Dropping it — the task finished, or was disposed — clears the waker it registered, so that a
/// [`StreamAck`] that outlives the task neither keeps that waker alive nor wakes it.
#[derive(Educe)]
#[educe(Debug)]
pub struct StreamAckedThenContext<C, I, M: ThreadMode> {
    #[educe(Debug(ignore))]
    state: Option<C>,
    #[educe(Debug(ignore))]
    permits: M::Ptr<Permits>,
    on_item: fn(&mut C, I, StreamAck<M>) -> bool,
    on_end: fn(C),
}

impl<C, I, M: ThreadMode> Drop for StreamAckedThenContext<C, I, M> {
    fn drop(&mut self) {
        // The permits outlive the task while a `StreamAck` does. A task disposed while it waits
        // for one left its waker there, which would keep the executor's task alive, and be woken
        // by the next `StreamAck` dropped. Taken under the lock, dropped after it: dropping a waker
        // can run arbitrary code of its executor.
        drop(self.permits.with_mut(|permits| permits.waker.take()));
    }
}

/// The acknowledgement of an element of [`Task::from_stream_acked_then`]: dropping it gives the
/// task its permit back, so that it polls its stream for another element.
///
/// It is not `Clone`, so that exactly one owner decides when the element is done. It is `Send`
/// when `M` is [`Shared`](crate::thread_mode::Shared): it may be dropped on any thread. Dropping
/// it after the task has finished does nothing.
#[derive(Educe)]
#[educe(Debug)]
pub struct StreamAck<M: ThreadMode>(#[educe(Debug(ignore))] M::Ptr<Permits>);

impl<M: ThreadMode> Drop for StreamAck<M> {
    fn drop(&mut self) {
        // A waker is there only while the task waits for a permit: none once the task is gone, so
        // a late acknowledgement only counts a permit nothing will use.
        let waker = self.0.with_mut(|permits| {
            permits.available += 1;
            permits.waker.take()
        });
        // Outside the lock: waking can run arbitrary code of the executor.
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// What the task and the [`StreamAck`]s it handed out share, behind one lock. A `StreamAck` keeps
/// it alive, so it may outlive the task.
struct Permits {
    /// How many more elements the task may take before one is acknowledged.
    available: usize,
    /// The waker of the task, registered while it waits for an acknowledgement: taken by the
    /// `StreamAck` that wakes it, or cleared by the task's context when the task is dropped.
    waker: Option<Waker>,
}

impl<C, I, M, SM> Task<StreamAckedThenContext<C, I, M>, SM>
where
    M: ThreadMode,
    SM: Stream<Item = I>,
{
    /// Drives `stream` to its end like [`from_stream_then`](Task::from_stream_then),
    /// `on_item(&mut state, item, ack)` for every element, then `on_end(state)`, except that the
    /// stream is polled only while a permit is left: `permits` at the start, one taken by every
    /// element, one given back by every [`StreamAck`] dropped, which wakes the task. The end of the
    /// stream is thus seen only once a permit is back, too.
    ///
    /// `on_item` returning `false` stops right there: the stream is not polled again, `on_end` is
    /// not called, and the stream and `state` are dropped with the task. There is a yield point
    /// after every element, even when the stream is always ready.
    ///
    /// `M` is the thread mode of the permits the task and its acknowledgements share: [`Shared`]
    /// when a `StreamAck` is dropped on another thread than the scheduler's, [`Local`] when
    /// everything stays on one thread.
    ///
    /// [`Shared`]: crate::thread_mode::Shared
    /// [`Local`]: crate::thread_mode::Local
    ///
    /// ```
    /// use futures::stream;
    /// use rx_rust::{
    ///     scheduler::{Scheduler, StreamAck, Task, virtual_time::VirtualTime},
    ///     thread_mode::{Shared, mutable::{MutableExt, MutableHelper}},
    /// };
    /// use std::{num::NonZeroUsize, sync::{Arc, Mutex}, time::Duration};
    ///
    /// let time = VirtualTime::new();
    /// // The elements taken, with the acknowledgements not given yet.
    /// let taken = Arc::new(Mutex::new(Vec::<(i32, StreamAck<Shared>)>::new()));
    /// let task = Task::from_stream_acked_then(
    ///     taken.clone(),
    ///     stream::iter([1, 2]),
    ///     NonZeroUsize::MIN,
    ///     |taken, item, ack| {
    ///         taken.with_mut(|taken| taken.push((item, ack)));
    ///         true
    ///     },
    ///     |_| {},
    /// );
    /// let _disposal = time.scheduler().run_task(task, None);
    ///
    /// time.advance_by(Duration::ZERO);
    /// assert_eq!(taken.with_ref(|taken| taken.len()), 1); // 2 waits for the acknowledgement of 1.
    ///
    /// drop(taken.take_value());
    /// time.advance_by(Duration::ZERO);
    /// assert_eq!(taken.with_ref(|taken| taken[0].0), 2);
    /// ```
    pub fn from_stream_acked_then(
        state: C,
        stream: SM,
        permits: NonZeroUsize,
        on_item: fn(&mut C, I, StreamAck<M>) -> bool,
        on_end: fn(C),
    ) -> Self {
        let context = StreamAckedThenContext {
            state: Some(state),
            permits: M::ptr(Permits {
                available: permits.get(),
                waker: None,
            }),
            on_item,
            on_end,
        };
        Self::with_pinned(context, stream, |context, stream, cx, _| {
            // The permit is taken before the stream is polled, in the same lock as the look, so
            // that an element costs one lock. Only a `StreamAck` adds a permit, and it takes the
            // waker under that lock too: one dropped after this look wakes the task.
            let (permitted, replaced) = context.permits.with_mut(|permits| {
                if let Some(left) = permits.available.checked_sub(1) {
                    permits.available = left;
                    return (true, None);
                }
                let replaced = match &permits.waker {
                    Some(waker) if waker.will_wake(cx.waker()) => None,
                    _ => permits.waker.replace(cx.waker().clone()),
                };
                (false, replaced)
            });
            // Outside the lock: dropping a waker can run arbitrary code of its executor.
            drop(replaced);
            if !permitted {
                return Next::Pending;
            }
            // The permit taken, as the acknowledgement of the element about to be polled. Dropping
            // it without an element, when the stream is pending or has ended, gives it back; it
            // wakes nothing, since the task registers its waker only when it gets no permit.
            let ack = StreamAck(context.permits.clone());
            let Poll::Ready(item) = stream.poll_next(cx) else {
                drop(ack);
                return Next::Pending;
            };
            match item {
                Some(item) => {
                    let state = context
                        .state
                        .as_mut()
                        .expect("from_stream_acked_then handler called after completion");
                    if (context.on_item)(state, item, ack) {
                        Next::Yield
                    } else {
                        Next::Finished
                    }
                }
                None => {
                    drop(ack);
                    let state = context
                        .state
                        .take()
                        .expect("from_stream_acked_then handler called after completion");
                    (context.on_end)(state);
                    Next::Finished
                }
            }
        })
    }
}
