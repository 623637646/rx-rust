use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;

/// Converts a `Future` of a `Result` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
///
/// An `Ok` is emitted as the single item, and the Observable then completes; an `Err` terminates it
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
///         observer::Termination,
///         operators::creating::from_try_future::FromTryFuture,
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
///     let subscription = FromTryFuture::new(async { Err::<i32, _>("boom") }, handle)
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
///     assert!(values.lock().unwrap().is_empty());
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Error("boom")]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromTryFuture<FU, S> {
    future: FU,
    scheduler: S,
}

impl<FU, S> FromTryFuture<FU, S> {
    pub fn new(future: FU, scheduler: S) -> Self {
        Self { future, scheduler }
    }
}

impl<T, E, FU, S> Observable<'static, T, E> for FromTryFuture<FU, S>
where
    FU: Future<Output = Result<T, E>> + MaybeSend + 'static,
    S: Scheduler,
{
    type D = S::D;

    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        self.scheduler.spawn_future(async {
            match self.future.await {
                Ok(value) => {
                    if observer.on_next(value).is_continue() {
                        observer.on_termination(Termination::Completed);
                    }
                }
                Err(error) => observer.on_termination(Termination::Error(error)),
            }
        })
    }
}
