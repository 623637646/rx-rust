use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription, observable::Observable, observer::Observer,
    scheduler::Scheduler,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

/// Creates an Observable that emits a sequence of integers spaced by a given time interval.
/// See <https://reactivex.io/documentation/operators/interval.html>
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {
/// #     panic!("Use tokio-scheduler feature to run tests.");
/// # }
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::observable_ext::ObservableExt,
///         observer::Termination,
///         operators::creating::interval::Interval,
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let subscription = Interval::new(Duration::from_millis(1), handle, None)
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
    pub fn new(period: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            period,
            scheduler,
            delay,
        }
    }
}

impl<'sub, S> Observable<'static, 'sub, usize, Infallible> for Interval<S>
where
    S: Scheduler,
{
    fn subscribe(
        self,
        mut observer: impl Observer<usize, Infallible> + NecessarySendSync + 'static,
    ) -> Subscription<'sub> {
        let disposal = self.scheduler.schedule_periodically(
            move |count| {
                observer.on_next(count);
                false
            },
            self.period,
            self.delay,
        );
        Subscription::new_with_disposal(disposal)
    }
}
