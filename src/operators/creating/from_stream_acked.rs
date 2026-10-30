//! The [`FromStreamAcked`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    scheduler::{Scheduler, SchedulerTypes, StreamAckedThenContext, Task},
};
use educe::Educe;
use futures::Stream;
use std::{convert::Infallible, num::NonZeroUsize};

/// The acknowledgement each item comes with, re-exported from [`scheduler`](crate::scheduler).
pub use crate::scheduler::StreamAck;

/// Converts a `Stream` into an observable that polls the stream for the next item only once an
/// earlier one is acknowledged.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// Each item comes with a [`StreamAck`], and dropping the `StreamAck` acknowledges the item. The
/// source holds a number of permits, one by default ([`with_permits`](Self::with_permits) sets
/// another): every item sent takes one, every acknowledgement gives one back, and the stream is
/// polled only while one is left. A downstream that keeps the `StreamAck` until it has handled the
/// item, asynchronously or not, thus sets the pace at which the stream is read. That is the
/// backpressure an observable otherwise lacks. The end of the stream is seen only when the stream
/// is polled too: the source completes once a permit is back, not as soon as the stream ends.
///
/// The usual way to handle the items is a `concat_map` that [`hold`]s the `StreamAck` in the
/// observable it makes of the item, so that the item is acknowledged once that observable has
/// ended, as in the example below. A `flat_map` in its place handles up to as many items at once
/// as there are permits. A `StreamAck` that is not held is dropped as soon as the closure returns:
/// nothing fails, but the stream is then read as fast as a `FromStream`, and the backpressure is
/// gone.
///
/// An operator that drops an item drops its `StreamAck` with it (`filter`, `skip`, `debounce`, …),
/// which acknowledges the item, so the stream keeps being read. An operator that holds items until
/// more arrive (`buffer_with_count`, `collect`, `zip`, `take_last`, a `ReplaySubject`, …) holds
/// their `StreamAck`s too: with fewer permits than the items it waits for, the source waits for it
/// while it waits for the source, and the pipeline stalls.
///
/// [`hold`]: crate::observable::ObservableExt::hold
///
/// # Examples
/// ```rust
/// use futures::stream;
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::creating::{from_stream_acked::FromStreamAcked, timer::Timer},
///     scheduler::virtual_time::VirtualTime,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let scheduler = time.scheduler();
/// let values = Arc::new(Mutex::new(Vec::new()));
///
/// // Each item takes 100 ms to handle; the stream is read only once the previous one is done.
/// let _subscription = FromStreamAcked::new(stream::iter([1, 2]), scheduler.clone())
///     .concat_map(move |(value, ack)| {
///         Timer::new(value * 10, Duration::from_millis(100), scheduler.clone()).hold(ack)
///     })
///     .subscribe_with_callback(
///         {
///             let values = values.clone();
///             move |value| values.with_mut(|values| values.push(value))
///         },
///         |_| {},
///     );
///
/// time.advance_by(Duration::from_millis(100));
/// assert_eq!(values.clone_value(), [10]);
/// time.advance_by(Duration::from_millis(100));
/// assert_eq!(values.clone_value(), [10, 20]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromStreamAcked<SM, S> {
    stream: SM,
    scheduler: S,
    permits: NonZeroUsize,
}

impl<SM, S> FromStreamAcked<SM, S> {
    /// Creates a [`FromStreamAcked`] with one permit: the stream is polled for an item only once
    /// the previous one is acknowledged.
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self::with_permits(stream, scheduler, NonZeroUsize::MIN)
    }

    /// Creates a [`FromStreamAcked`] that sends up to `permits` items not acknowledged yet.
    pub fn with_permits(stream: SM, scheduler: S, permits: NonZeroUsize) -> Self {
        Self {
            stream,
            scheduler,
            permits,
        }
    }
}

impl<T, SM, S> ObservableTypes for FromStreamAcked<SM, S>
where
    SM: Stream<Item = T>,
    S: SchedulerTypes,
{
    type Item = (T, StreamAck<S::Mode>);
    type Error = Infallible;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, SM, S, OR> Observable<OR> for FromStreamAcked<SM, S>
where
    OR: Observer<(T, StreamAck<<S as SchedulerTypes>::Mode>), Infallible>,
    SM: Stream<Item = T>,
    S: Scheduler<StreamAckedThenContext<OR, T, <S as SchedulerTypes>::Mode>, SM>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::from_stream_acked_then(
            observer,
            self.stream,
            self.permits,
            // The observer ending its own stream stops polling it, as for `FromStream`.
            |observer, value, ack| observer.on_next((value, ack)).is_continue(),
            |observer| observer.on_termination(Termination::Completed),
        );
        self.scheduler.run_task(task, None)
    }
}
