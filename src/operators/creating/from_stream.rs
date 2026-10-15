//! The [`FromStream`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    scheduler::{Scheduler, SchedulerTypes, StreamThenContext, Task},
};
use educe::Educe;
use futures::Stream;
use std::convert::Infallible;

/// Converts a `Stream` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use futures::stream;
///     use rx_rust::{
///         observable::ObservableExt,
///         observer::Termination,
///         operators::creating::from_stream::FromStream,
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use std::{
///         sync::{Arc, Mutex},
///         time::Duration,
///     };
///
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///
///     let subscription = FromStream::new(stream::iter([10, 20]), TokioScheduler::current())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             |termination| assert_eq!(termination, Termination::Completed),
///         );
///
///     tokio::time::sleep(Duration::from_millis(20)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[10, 20]);
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromStream<SM, S> {
    /// Creates a [`FromStream`].
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self { stream, scheduler }
    }
}

impl<T, SM, S> ObservableTypes for FromStream<SM, S>
where
    SM: Stream<Item = T>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = Infallible;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, SM, S, OR> Observable<OR> for FromStream<SM, S>
where
    OR: Observer<T, Infallible>,
    SM: Stream<Item = T>,
    S: Scheduler<StreamThenContext<OR, T>, SM>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::from_stream_then(
            observer,
            self.stream,
            // The observer ending its own stream stops polling it, so an infinite stream is not
            // driven for values that have nothing to be delivered to; the task then drops the
            // observer, like a disposed one.
            |observer, value| observer.on_next(value).is_continue(),
            |observer| observer.on_termination(Termination::Completed),
        );
        self.scheduler.run_task(task, None)
    }
}
