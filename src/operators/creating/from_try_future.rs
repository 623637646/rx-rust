//! The [`FromTryFuture`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    scheduler::{FutureThenContext, Scheduler, SchedulerTypes, Task},
};
use educe::Educe;

/// Converts a `Future` of a `Result` into an observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// An `Ok` is emitted as the single item, and the observable then completes; an `Err` terminates it
/// with that error. This is how a one-shot result, the `Single` of ReactiveX, enters a pipeline.
/// A future that cannot fail goes through
/// [`FromFuture`](crate::operators::creating::from_future::FromFuture) instead.
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
///         operators::creating::from_try_future::FromTryFuture,
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///
///     let result = FromTryFuture::new(async { Err::<i32, _>("boom") }, TokioScheduler::current())
///         .into_try_future()
///         .await;
///     assert_eq!(result, Err("boom"));
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromTryFuture<FU, S> {
    future: FU,
    scheduler: S,
}

impl<FU, S> FromTryFuture<FU, S> {
    /// Creates a [`FromTryFuture`].
    pub fn new(future: FU, scheduler: S) -> Self {
        Self { future, scheduler }
    }
}

impl<T, E, FU, S> ObservableTypes for FromTryFuture<FU, S>
where
    FU: Future<Output = Result<T, E>>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, E, FU, S, OR> Observable<OR> for FromTryFuture<FU, S>
where
    OR: Observer<T, E>,
    FU: Future<Output = Result<T, E>>,
    S: Scheduler<FutureThenContext<OR, Result<T, E>>, FU>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task =
            Task::from_future_then(observer, self.future, |mut observer, result| match result {
                Ok(value) => {
                    if observer.on_next(value).is_continue() {
                        observer.on_termination(Termination::Completed);
                    }
                }
                Err(error) => observer.on_termination(Termination::Error(error)),
            });
        self.scheduler.run_task(task, None)
    }
}
