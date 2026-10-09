//! The [`SubscribeOn`] operator, behind
//! [`ObservableExt::subscribe_on`](crate::observable::ObservableExt::subscribe_on).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        shared_disposal::SharedDisposal,
    },
    observable::{Observable, ObservableTypes},
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
///
/// The whole source is subscribed from a task of `scheduler`, together with the schedulers its
/// own operators hold. A `Local` one among them is bound to the thread it was made on, so moving
/// the subscription to a `Shared` scheduler is refused at compile time:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     scheduler::runtime::tokio::{TokioLocalScheduler, TokioScheduler},
/// };
/// use std::time::Duration;
///
/// let _subscription = Just::new(1)
///     .delay(Duration::from_millis(5), TokioLocalScheduler::ambient())
///     .subscribe_on(TokioScheduler::current())
///     .subscribe_with_callback(|_| {}, |_| {});
/// ```
///
/// A `Shared` one moves with it:
///
/// ```no_run
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     scheduler::runtime::tokio::{TokioLocalScheduler, TokioScheduler},
/// };
/// use std::time::Duration;
///
/// let _subscription = Just::new(1)
///     .delay(Duration::from_millis(5), TokioScheduler::current())
///     .subscribe_on(TokioScheduler::current())
///     .subscribe_with_callback(|_| {}, |_| {});
/// ```
///
/// The events then come from `scheduler`'s thread even when the source emits synchronously, so the
/// mode of a `SubscribeOn` joins the source's and the scheduler's. Merged with a source that emits
/// on the subscribing thread, it therefore makes the merge `Shared`, whose state both threads
/// reach:
///
/// ```no_run
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     scheduler::runtime::tokio::TokioScheduler,
/// };
///
/// let _subscription = Just::new(1)
///     .subscribe_on(TokioScheduler::current())
///     .merge_with(Just::new(2))
///     .subscribe_with_callback(|_| {}, |_| {});
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
    ChainDisposal<SD, SharedDisposal<M, DisposeOnDrop<D>>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

/// The thread mode of a [`SubscribeOn`]: relative to the thread that subscribes, the events come
/// from the scheduler's thread, where the subscription is moved, and the source may move them
/// again.
///
/// It is not the source's mode alone: `Just.subscribe_on(pool).merge(Just)` would then share the
/// state of `merge` through an `Rc`, which both the pool's thread and the subscribing one reach.
type SubscribeOnMode<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The slot the task puts the source's subscription into once it has subscribed.
type UpstreamSlot<OE, S> =
    SharedDisposal<SubscribeOnMode<OE, S>, DisposeOnDrop<<OE as ObservableTypes>::Disposal>>;

/// The task of a [`SubscribeOn`]: the source, the observer, and where to put the subscription.
type SubscribeOnTask<OE, OR, S> = OnceContext<(OE, OR, UpstreamSlot<OE, S>)>;

impl<T, E, OE, S> ObservableTypes for SubscribeOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = SubscribeOnMode<OE, S>;
    /// First the task that subscribes, then the subscription it made.
    type Disposal = Disposal<SubscribeOnMode<OE, S>, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for SubscribeOn<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    S: Scheduler<SubscribeOnTask<OE, OR, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
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
