//! The [`FromTryStream`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    scheduler::{Scheduler, SchedulerTypes, StreamThenContext, Task},
};
use educe::Educe;
use futures::Stream;

/// Converts a `Stream` of `Result`s into an observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// Each `Ok` is emitted as an item. The first `Err` terminates the observable with that error and
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
///     use rx_rust::{
///         observable::ObservableExt,
///         operators::creating::from_try_stream::FromTryStream,
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use futures::{stream, StreamExt};
///
///     let stream = stream::iter([Ok(10), Ok(20), Err("boom"), Ok(30)]);
///     let values = FromTryStream::new(stream, TokioScheduler::current())
///         .into_try_stream()
///         .collect::<Vec<_>>()
///         .await;
///     assert_eq!(values, [Ok(10), Ok(20), Err("boom")]);
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
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::from_stream_then(
            Some(observer),
            self.stream,
            |observer, result| match result {
                // The observer ending its own stream stops polling it, so an infinite stream is
                // not driven for values that have nothing to be delivered to.
                Ok(value) => observer
                    .as_mut()
                    .is_some_and(|observer| observer.on_next(value).is_continue()),
                // An error is terminal for an observable, so the stream is left wherever it is.
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
