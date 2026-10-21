//! The [`Timer`] source.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
    scheduler::{OnceContext, Scheduler, SchedulerTypes, Task},
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

/// Creates an observable that emits a single item after a given delay.
/// See <https://reactivex.io/documentation/operators/timer.html>
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
///         operators::creating::timer::Timer,
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use std::time::Duration;
///
///     let value = Timer::new("tick", Duration::from_millis(5), TokioScheduler::current())
///         .into_future()
///         .await;
///     assert_eq!(value, Some("tick"));
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timer<T, S> {
    value: T,
    delay: Duration,
    scheduler: S,
}

impl<T, S> Timer<T, S> {
    /// Creates a [`Timer`].
    pub fn new(value: T, delay: Duration, scheduler: S) -> Self {
        Self {
            value,
            delay,
            scheduler,
        }
    }
}

impl<T, S> ObservableTypes for Timer<T, S>
where
    S: SchedulerTypes,
{
    type Item = T;
    type Error = Infallible;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<T, S, OR> Observable<OR> for Timer<T, S>
where
    OR: Observer<T, Infallible>,
    S: Scheduler<OnceContext<(OR, T)>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let task = Task::once((observer, self.value), |(mut observer, value)| {
            if observer.on_next(value).is_continue() {
                observer.on_termination(Termination::Completed);
            }
        });
        self.scheduler.run_task(task, Some(self.delay))
    }
}
