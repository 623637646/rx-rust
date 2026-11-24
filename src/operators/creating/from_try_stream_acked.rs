//! The [`FromTryStreamAcked`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Scheduler, SchedulerTypes, StreamAck, StreamAckedThenContext, Task},
};
use educe::Educe;
use futures::Stream;
use std::num::NonZeroUsize;

/// Converts a `Stream` of `Result`s into an observable with a limit on unacknowledged items.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// Each `Ok` is emitted as an item, with the [`StreamAck`] that acknowledges it, as for
/// [`FromStreamAcked`](super::from_stream_acked::FromStreamAcked), which describes the permits.
/// The first `Err` terminates the observable with that error and the stream is not polled any
/// further, as for [`FromTryStream`](super::from_try_stream::FromTryStream).
///
/// # Examples
/// ```rust
/// use futures::stream;
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::{from_result::FromResult, from_try_stream_acked::FromTryStreamAcked},
///     scheduler::virtual_time::VirtualTime,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let termination = Arc::new(Mutex::new(None));
///
/// let stream = stream::iter([Ok(10), Ok(20), Err("boom"), Ok(30)]);
/// let _subscription = FromTryStreamAcked::new(stream, time.scheduler())
///     .concat_map(|(value, ack)| FromResult::new(Ok(value * 2)).hold(ack))
///     .subscribe_with_callback(
///         {
///             let values = values.clone();
///             move |value| values.with_mut(|values| values.push(value))
///         },
///         {
///             let termination = termination.clone();
///             move |t| termination.with_mut(|termination| *termination = Some(t))
///         },
///     );
///
/// time.advance_by(Duration::ZERO);
/// assert_eq!(values.clone_value(), [20, 40]);
/// assert_eq!(termination.clone_value(), Some(Termination::Error("boom")));
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromTryStreamAcked<SM, S> {
    stream: SM,
    scheduler: S,
    permits: NonZeroUsize,
}

impl<SM, S> FromTryStreamAcked<SM, S> {
    /// Creates a [`FromTryStreamAcked`] with one permit: the stream is polled for an item only once
    /// the previous one is acknowledged.
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self {
            stream,
            scheduler,
            permits: NonZeroUsize::MIN,
        }
    }

    /// Sets the maximum number of unacknowledged items, one by default.
    pub fn permits(self, permits: NonZeroUsize) -> Self {
        Self { permits, ..self }
    }
}

impl<T, E, SM, S> ObservableTypes for FromTryStreamAcked<SM, S>
where
    SM: Stream<Item = Result<T, E>>,
    S: SchedulerTypes,
{
    type Item = (T, StreamAck<S::Mode>);
    type Error = E;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, E, SM, S, OR> Observable<OR> for FromTryStreamAcked<SM, S>
where
    OR: Observer<(T, StreamAck<<S as SchedulerTypes>::Mode>), E>,
    SM: Stream<Item = Result<T, E>>,
    S: Scheduler<StreamAckedThenContext<Option<OR>, Result<T, E>, <S as SchedulerTypes>::Mode>, SM>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::from_stream_acked_then(
            Some(observer),
            self.stream,
            self.permits,
            |observer, result, ack| match result {
                // The observer ending its own stream stops polling it, as for `FromTryStream`.
                Ok(value) => observer
                    .as_mut()
                    .map_or(Flow::Break(()), |observer| observer.on_next((value, ack))),
                // An error is terminal for an observable, so the stream is left wherever it is;
                // the `StreamAck` is dropped, which gives back a permit nothing will use.
                Err(error) => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Error(error));
                    }
                    Flow::Break(())
                }
            },
            |observer| {
                if let Some(observer) = observer {
                    observer.on_termination(Termination::Completed);
                }
            },
        );
        self.scheduler.run_task(task, None)
    }
}
