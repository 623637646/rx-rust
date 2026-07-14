use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::convert::Infallible;

/// Converts a Future into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
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
///         operators::creating::from_future::FromFuture,
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let handle = tokio::runtime::Handle::current();
///
///     let subscription = FromFuture::new(async { 7 }, handle).subscribe_with_callback(
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
    pub fn new(future: FU, scheduler: S) -> Self {
        Self { future, scheduler }
    }
}

impl<T, FU, S> Observable<'static> for FromFuture<FU, S>
where
    FU: Future<Output = T> + MaybeSend + 'static,
    S: Scheduler,
{
    type T = T;
    type E = Infallible;
    type D = S::D;

    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        self.scheduler.spawn_future(async {
            let result = self.future.await;
            observer.on_next(result);
            observer.on_termination(Termination::Completed);
        })
    }
}
