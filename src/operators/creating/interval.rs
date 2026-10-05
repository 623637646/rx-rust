//! The [`Interval`] source.

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
    scheduler::{PeriodicContext, Scheduler, SchedulerTypes, Task},
};
use educe::Educe;
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

/// Creates an Observable that emits a sequence of integers spaced by a given time interval.
/// See <https://reactivex.io/documentation/operators/interval.html>
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
///         operators::creating::interval::Interval,
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let subscription = Interval::new(Duration::from_millis(1), scheduler, None)
///         .take(3)
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[0, 1, 2]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Interval<S> {
    period: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<S> Interval<S> {
    /// Creates an [`Interval`].
    pub fn new(period: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            period,
            scheduler,
            delay,
        }
    }
}

impl<S> ObservableTypes for Interval<S>
where
    S: SchedulerTypes,
{
    type Item = usize;
    type Error = Infallible;
    type Mode = S::Mode;
    type D = S::D;
}

impl<S, OR> Observable<OR> for Interval<S>
where
    OR: Observer<usize, Infallible>,
    S: Scheduler<PeriodicContext<OR>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let task = Task::periodic(
            observer,
            // The answer is what keeps the schedule running, so an observer that stopped ends it:
            // nothing is completed, since an interval never completes anyway.
            |observer, count| observer.on_next(count).is_continue(),
            self.period,
            // Fixed-rate, anchored to the time of the subscription plus the delay.
            Some(Instant::now() + self.delay.unwrap_or_default()),
        );
        self.scheduler.run_task(task, self.delay)
    }
}
