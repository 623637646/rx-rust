use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use futures::Stream;

/// Converts a `Stream` of `Result`s into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// Each `Ok` is emitted as an item. The first `Err` terminates the Observable with that error and
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
///     use futures::stream;
///     use rx_rust::{
///         observable::ObservableExt,
///         observer::Termination,
///         operators::creating::from_try_stream::FromTryStream,
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let stream = stream::iter([Ok(10), Ok(20), Err("boom"), Ok(30)]);
///
///     let subscription = FromTryStream::new(stream, handle).subscribe_with_callback(
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
///         &[Termination::Error("boom")]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromTryStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromTryStream<SM, S> {
    pub fn new(stream: SM, scheduler: S) -> Self {
        Self { stream, scheduler }
    }
}

impl<T, E, SM, S> Observable<'static, T, E> for FromTryStream<SM, S>
where
    SM: Stream<Item = Result<T, E>> + MaybeSend + 'static,
    S: Scheduler,
{
    type D = S::D;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let mut observer = Some(observer);
        self.scheduler
            .schedule_stream(self.stream, move |result| match result {
                Some(Ok(value)) => {
                    let flow = match observer.as_mut() {
                        Some(observer) => observer.on_next(value),
                        None => Flow::Stop,
                    };
                    if flow.is_stop() {
                        // The observer ended its own stream: release it here and tell the
                        // scheduler to stop polling the stream, so an infinite one is not driven
                        // for values that have nothing to be delivered to.
                        drop(observer.take());
                    }
                    flow.is_continue()
                }
                // An error is terminal for an Observable, so the stream is left wherever it is.
                Some(Err(error)) => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Error(error))
                    }
                    false
                }
                None => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Completed)
                    }
                    false
                }
            })
    }
}
