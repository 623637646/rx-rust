//! The [`FromTryStream`] source.

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Observer, Termination},
    scheduler::{Scheduler, SchedulerTypes, StreamThenContext, Task},
};
use educe::Educe;
use futures::Stream;

/// Converts a `Stream` of `Result`s into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// Each `Ok` is emitted as an item. The first `Err` terminates the Observable with that error and
/// the stream is not polled any further, so whatever it would have yielded after the error is
/// never seen. A stream that cannot fail goes through
/// [`FromStream`](crate::operators::creating::from_stream::FromStream) instead.
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
///         operators::creating::from_try_stream::FromTryStream,
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let stream = stream::iter([Ok(10), Ok(20), Err("boom"), Ok(30)]);
///
///     let subscription = FromTryStream::new(stream, scheduler).subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[10, 20]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Error("boom")]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromTryStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromTryStream<SM, S> {
    /// Creates a [`FromTryStream`].
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self { stream, scheduler }
    }
}

impl<T, E, SM, S> ObservableTypes for FromTryStream<SM, S>
where
    SM: Stream<Item = Result<T, E>>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, E, SM, S, OR> Observable<OR> for FromTryStream<SM, S>
where
    OR: Observer<T, E>,
    SM: Stream<Item = Result<T, E>>,
    S: Scheduler<StreamThenContext<Option<OR>, Result<T, E>>, SM>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let task = Task::from_stream_then(
            Some(observer),
            self.stream,
            |observer, result| match result {
                // The observer ending its own stream stops polling it, so an infinite stream is
                // not driven for values that have nothing to be delivered to.
                Ok(value) => observer
                    .as_mut()
                    .is_some_and(|observer| observer.on_next(value).is_continue()),
                // An error is terminal for an Observable, so the stream is left wherever it is.
                Err(error) => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Error(error));
                    }
                    false
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
