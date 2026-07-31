use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use futures::Stream;
use std::convert::Infallible;

/// Converts a `Stream` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
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
///         operators::creating::from_stream::FromStream,
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let stream = stream::iter([10, 20]);
///
///     let subscription = FromStream::new(stream, handle).subscribe_with_callback(
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
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromStream<SM, S> {
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self { stream, scheduler }
    }
}

impl<T, SM, S> Observable<'static, T, Infallible> for FromStream<SM, S>
where
    SM: Stream<Item = T> + MaybeSend + 'static,
    S: Scheduler,
{
    type D = S::D;

    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let mut observer = Some(observer);
        self.scheduler
            .schedule_stream(self.stream, move |result| match result {
                Some(value) => {
                    if let Some(observer) = observer.as_mut() {
                        observer.on_next(value)
                    }
                }
                None => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Completed)
                    }
                }
            })
    }
}
