//! The [`Interval`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    scheduler::{PeriodicContext, Scheduler, SchedulerTypes, Task},
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

/// Creates an Observable that emits a sequence of integers spaced by a given time interval.
/// See <https://reactivex.io/documentation/operators/interval.html>
///
/// [`new`](Self::new) emits `0` one `period` after the subscription, as ReactiveX does, and
/// [`with_initial_delay`](Self::with_initial_delay) after a delay of its own, `Duration::ZERO` to
/// start at once. Every following count comes one `period` later, at a fixed rate. It never
/// completes.
///
/// "At once" is as soon as the scheduler runs the task, not inside `subscribe`: on a
/// [`VirtualTime`](crate::scheduler::virtual_time::VirtualTime), the
/// `advance_by(Duration::ZERO)` that follows.
///
/// A time too far out for an `Instant` to represent never comes. An initial delay that long:
/// nothing is emitted until the subscription is disposed. A `period` that long: `0` is emitted,
/// then the observer is dropped without a termination, as [`Never`](super::never::Never) drops it.
///
/// # Panics
///
/// Subscribing panics if `period` is zero.
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
///     let subscription = Interval::new(Duration::from_millis(1), scheduler)
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
    initial_delay: Duration,
    period: Duration,
    scheduler: S,
}

impl<S> Interval<S> {
    /// Creates an [`Interval`] that emits `0` after one `period`, then every `period`.
    pub fn new(period: Duration, scheduler: S) -> Self {
        Self::with_initial_delay(period, period, scheduler)
    }

    /// Creates an [`Interval`] that emits `0` after `initial_delay`, then every `period`.
    /// `Duration::ZERO` starts at once.
    pub fn with_initial_delay(initial_delay: Duration, period: Duration, scheduler: S) -> Self {
        Self {
            initial_delay,
            period,
            scheduler,
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
    type Disposal = S::Disposal;
}

impl<S, OR> Observable<OR> for Interval<S>
where
    OR: Observer<usize, Infallible>,
    S: Scheduler<PeriodicContext<OR>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::periodic(
            observer,
            // The answer is what keeps the schedule running, so an observer that stopped ends it:
            // nothing is completed, since an interval never completes anyway.
            |observer, count| observer.on_next(count).is_continue(),
            self.period,
            // Fixed-rate, anchored to the time of the subscription plus the initial delay. A delay
            // too long for an `Instant` never ends: the first step never comes, so it needs no
            // anchor.
            self.scheduler.now().checked_add(self.initial_delay),
        );
        let delay = (!self.initial_delay.is_zero()).then_some(self.initial_delay);
        self.scheduler.run_task(task, delay)
    }
}
