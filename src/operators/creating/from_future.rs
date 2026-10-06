//! The [`FromFuture`] source.

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Observer, Termination},
    scheduler::{FutureThenContext, Scheduler, SchedulerTypes, Task},
};
use educe::Educe;
use std::convert::Infallible;

/// Converts a Future into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// The output is emitted as the single item, and the Observable then completes. A future of a
/// `Result` goes through
/// [`FromTryFuture`](crate::operators::creating::from_try_future::FromTryFuture) instead, which
/// turns its `Err` into the error of the Observable.
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
///         observer::Termination,
///         operators::creating::from_future::FromFuture,
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///
///     let subscription = FromFuture::new(async { 7 }, scheduler).subscribe_with_callback(
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
///     assert_eq!(&*values.lock().unwrap(), &[7]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromFuture<FU, S> {
    future: FU,
    scheduler: S,
}

impl<FU, S> FromFuture<FU, S> {
    /// Creates a [`FromFuture`].
    pub fn new(future: FU, scheduler: S) -> Self {
        Self { future, scheduler }
    }
}

impl<T, FU, S> ObservableTypes for FromFuture<FU, S>
where
    FU: Future<Output = T>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = Infallible;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, FU, S, OR> Observable<OR> for FromFuture<FU, S>
where
    OR: Observer<T, Infallible>,
    FU: Future<Output = T>,
    S: Scheduler<FutureThenContext<OR, T>, FU>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let task = Task::from_future_then(observer, self.future, |mut observer, value| {
            if observer.on_next(value).is_continue() {
                observer.on_termination(Termination::Completed);
            }
        });
        self.scheduler.run_task(task, None)
    }
}
