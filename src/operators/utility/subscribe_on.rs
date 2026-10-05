//! The [`SubscribeOn`] operator, behind
//! [`ObservableExt::subscribe_on`](crate::observable::ObservableExt::subscribe_on).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal},
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
    scheduler::{OnceContext, Scheduler, SchedulerTypes, Task},
    thread_mode::{Joined, ThreadMode},
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will subscribe to this Observable.
/// See <https://reactivex.io/documentation/operators/subscribeon.html>
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
///         operators::{
///             creating::from_iter::FromIter,
///             utility::subscribe_on::SubscribeOn,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = SubscribeOn::new(FromIter::new(vec![1, 2, 3]), scheduler.clone())
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
    /// Creates a [`SubscribeOn`] over `source`;
    /// [`ObservableExt::subscribe_on`](crate::observable::ObservableExt::subscribe_on) is the
    /// fluent form.
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

delegate_disposal!(
    Disposal<M, SD, D>,
    ChainDisposal<SD, SharedDisposal<M, Subscription<D>>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

/// The thread mode of a [`SubscribeOn`]: relative to the thread that subscribes, the events come
/// from the scheduler's thread, where the subscription is moved, and the source may move them
/// again.
///
/// It is not the source's mode alone: `Just.subscribe_on(pool).merge(Just)` would then share the
/// state of `merge` through an `Rc`, which both the pool's thread and the subscribing one reach.
pub type SubscribeOnMode<OE, S> =
    Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The slot the task puts the source's subscription into once it has subscribed.
pub type UpstreamSlot<OE, S> =
    SharedDisposal<SubscribeOnMode<OE, S>, Subscription<<OE as ObservableTypes>::D>>;

/// The task of a [`SubscribeOn`]: the source, the observer, and where to put the subscription.
pub type SubscribeOnTask<OE, OR, S> = OnceContext<(OE, OR, UpstreamSlot<OE, S>)>;

impl<T, E, OE, S> ObservableTypes for SubscribeOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = SubscribeOnMode<OE, S>;
    /// First the task that subscribes, then the subscription it made.
    type D = Disposal<SubscribeOnMode<OE, S>, S::D, OE::D>;
}

impl<T, E, OE, S, OR> Observable<OR> for SubscribeOn<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    S: Scheduler<SubscribeOnTask<OE, OR, S>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        // The subscription happens later, and the disposal may come first: the slot then hands
        // the subscription back to be disposed at once.
        let upstream_slot = UpstreamSlot::<OE, S>::default();
        let task = Task::once(
            (self.source, observer, upstream_slot.clone()),
            |(source, observer, upstream_slot)| {
                upstream_slot.replace(|| source.subscribe(observer));
            },
        );
        let disposal = self.scheduler.run_task(task, None);
        disposal.then(upstream_slot).map_into()
    }
}
