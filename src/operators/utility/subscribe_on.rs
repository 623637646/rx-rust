//! The [`SubscribeOn`] operator, behind
//! [`ObservableExt::subscribe_on`](crate::observable::ObservableExt::subscribe_on).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        replaceable_disposal::ReplaceableDisposal,
    },
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    scheduler::{OnceContext, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will subscribe to this observable.
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
///         operators::{creating::from_iter::FromIter, utility::subscribe_on::SubscribeOn},
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use futures::StreamExt;
///
///     let values = SubscribeOn::new(FromIter::new([1, 2, 3]), TokioScheduler::current())
///         .into_stream()
///         .collect::<Vec<_>>()
///         .await;
///     assert_eq!(values, [1, 2, 3]);
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
    ChainDisposal<SD, ReplaceableDisposal<M, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

/// The slot the task puts the source's subscription into once it has subscribed.
type UpstreamSlot<OE, S> =
    ReplaceableDisposal<JoinedSchedulerModeOf<OE, S>, <OE as ObservableTypes>::Disposal>;

/// The task of a [`SubscribeOn`]: the source, the observer, and where to put the subscription.
type SubscribeOnTask<OE, OR, S> = OnceContext<(OE, OR, UpstreamSlot<OE, S>)>;

impl<T, E, OE, S> ObservableTypes for SubscribeOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// Relative to the thread that subscribes, the events come from the scheduler's thread, where
    /// the subscription is moved, and the source may move them again, so the mode is both joined.
    ///
    /// It is not the source's mode alone: `Just.subscribe_on(pool).merge_with(Just)` would then
    /// share the state of `merge` through an `Rc`, which both the pool's thread and the subscribing
    /// one reach.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    /// First the task that subscribes, then the subscription it made.
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, S::Disposal, OE::Disposal>;
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
                upstream_slot.replace_with(|| source.subscribe(observer));
            },
        );
        let disposal = self.scheduler.run_task(task, None);
        disposal.then(upstream_slot).map_inner_into()
    }
}
