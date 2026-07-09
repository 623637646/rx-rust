use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

/// Creates an Observable that emits a single item after a given delay.
/// See <https://reactivex.io/documentation/operators/timer.html>
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
///         observable::ObservableExt,
///         observer::Termination,
///         operators::creating::timer::Timer,
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
///
///     let subscription = Timer::new("tick", Duration::from_millis(5), handle)
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
///     assert_eq!(&*values.lock().unwrap(), &["tick"]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
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
    pub fn new(value: T, delay: Duration, scheduler: S) -> Self {
        Self {
            value,
            delay,
            scheduler,
        }
    }
}

impl<T, S> Observable<'static, T, Infallible> for Timer<T, S>
where
    T: MaybeSend + 'static,
    S: Scheduler,
{
    type D = S::D;

    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        self.scheduler.schedule(
            || {
                observer.on_next(self.value);
                observer.on_termination(Termination::Completed);
            },
            Some(self.delay),
        )
    }
}
