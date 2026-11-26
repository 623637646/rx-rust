//! Gathers items into `Vec`s bounded by both a time span and a length.
//!
//! The [`BufferWithTimeOrCount`] operator, behind
//! [`ObservableExt::buffer_with_time_or_count`](crate::observable::ObservableExt::buffer_with_time_or_count).

use super::internal::{
    bundle::Bundles,
    overlap::NoOverlap,
    time_or_count::{self, TimeOrCountObserver, TimeOrCountTask},
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    scheduler::{Scheduler, SchedulerTypes},
    thread_mode::ThreadMode,
    utils::subscribe_with_context,
};
use educe::Educe;
use std::{num::NonZeroUsize, time::Duration};

/// Gathers items from an observable into bundles and emits these bundles as `Vec<T>`, either when
/// the bundle reaches `count` items or every `time_span`, whichever happens first.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The timer first fires one `time_span` after the subscription, as ReactiveX does. A bundle
/// emitted because it is full restarts the timer, so the next timed bundle comes `time_span` after
/// it.
/// A late timer cuts overdue bundles without waiting another span, unless a full bundle reset it.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long,
/// bundles are emitted by count and by the completion only.
///
/// # Panics
///
/// Subscribing panics if `time_span` is zero.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let span = Duration::from_millis(5);
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = values.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription = BufferWithTimeOrCount::new(
///     source.clone(), std::num::NonZeroUsize::new(2).unwrap(), span, time.scheduler(),
/// )
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2); // Full: emitted immediately.
/// let _ = source.on_next(3);
/// assert_eq!(values.clone_value(), [vec![1, 2]]);
/// time.advance_by(span);
/// assert_eq!(values.clone_value(), [vec![1, 2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeOrCount<OE, S> {
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    /// Creates a [`BufferWithTimeOrCount`] over `source`;
    /// [`ObservableExt::buffer_with_time_or_count`](crate::observable::ObservableExt::buffer_with_time_or_count)
    /// is the fluent form.
    pub fn new(source: OE, count: NonZeroUsize, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
        }
    }
}

/// The observer a [`BufferWithTimeOrCount`] gives its source.
pub(crate) type BufferWithTimeOrCountObserver<M, T, E, OR, D, C> =
    TimeOrCountObserver<M, Bundles<T, NoOverlap>, Vec<T>, E, OR, D, C>;

/// The task of a [`BufferWithTimeOrCount`] timer. It holds the context strongly, so that the
/// bundles keep being cut after the source has let go of its observer without a termination, until
/// the subscription is disposed.
type BufferWithTimeOrCountTask<T, E, OR, OE, S> =
    TimeOrCountTask<OE, S, Bundles<T, NoOverlap>, Vec<T>, E, OR>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        Vec<T>,
        E,
        time_or_count::Model<Bundles<T, NoOverlap>>,
        ChainDisposal<SD, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTimeOrCount<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    /// The timer's thread and the source's both emit buffers, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTimeOrCount<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeOrCountObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
                <S as SchedulerTypes>::Clock,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeOrCountTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // One bundle at a time, so no item is cloned; the first opens with room for `count` items,
        // each after it with room for as many as the last one emitted.
        time_or_count::subscribe(
            self.source,
            self.count,
            self.time_span,
            self.scheduler,
            Bundles::with_room(NoOverlap, self.count.get()),
            observer,
        )
        .map_inner_into()
    }
}
