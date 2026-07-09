use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::Observer,
    scheduler::Scheduler,
    utils::types::{MaybeSend, Mutable, MutableHelper, Shared},
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will subscribe to this Observable.
/// See <https://reactivex.io/documentation/operators/subscribeon.html>
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
///         operators::{
///             creating::from_iter::FromIter,
///             utility::subscribe_on::SubscribeOn,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = SubscribeOn::new(FromIter::new(vec![1, 2, 3]), handle.clone())
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
///     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscribeOn<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> SubscribeOn<OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for SubscribeOn<OE, S>
where
    OE: Observable<'or, 'static, T, E> + MaybeSend + 'static,
    S: Scheduler,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'static) -> Subscription<'sub> {
        let sub = Shared::new(Mutable::new(Some(Subscription::default()))); // Placeholder
        let sub_cloned = sub.clone();
        let disposal = self.scheduler.schedule(
            move || {
                sub_cloned.lock_mut(|mut lock| {
                    if lock.is_some() {
                        // Only subscribe if not unsubscribed yet.
                        let sub = self.source.subscribe(observer);
                        lock.replace(sub);
                    }
                });
            },
            None,
        );
        Subscription::new_with_disposal(sub) + disposal
    }
}
