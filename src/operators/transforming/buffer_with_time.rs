//! Gathers items into `Vec`s, one per time span.
//!
//! The [`BufferWithTime`] operator, behind
//! [`ObservableExt::buffer_with_time`](crate::observable::ObservableExt::buffer_with_time).

use super::internal::{
    bundle::Bundles,
    overlap::NoOverlap,
    time_shift::{self, TimeShiftObserver, TimeShiftTask},
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
use std::time::Duration;

/// Periodically gathers items from an observable into bundles and emits these bundles as `Vec<T>`,
/// every `time_span`.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The first bundle is emitted one `time_span` after the subscription, as ReactiveX does, and
/// every following one `time_span` later, at a fixed rate, empty or not. On completion, a pending
/// bundle with items is emitted before the completion.
/// A late timer cuts overdue bundles without waiting another span, keeping the original rate.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long, no
/// bundle is emitted by time, and what is gathered is emitted by the completion.
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
///     operators::transforming::buffer_with_time::BufferWithTime,
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
/// let _subscription = BufferWithTime::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// time.advance_by(span);
/// assert_eq!(values.clone_value(), [vec![1, 2]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> BufferWithTime<OE, S> {
    /// Creates a [`BufferWithTime`] over `source`;
    /// [`ObservableExt::buffer_with_time`](crate::observable::ObservableExt::buffer_with_time) is
    /// the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

/// The observer a [`BufferWithTime`] gives its source: that of a
/// [`BufferWithTimeAndShift`](super::buffer_with_time_and_shift::BufferWithTimeAndShift) whose
/// bundles never overlap, so that it does not ask its items to be [`Clone`].
pub(crate) type BufferWithTimeObserver<M, T, E, OR, D> =
    TimeShiftObserver<M, Bundles<T, NoOverlap>, Vec<T>, E, OR, D>;

/// The task of a [`BufferWithTime`] timer. It holds the context strongly, so that the bundles keep
/// being cut after the source has let go of its observer without a termination, until the
/// subscription is disposed.
type BufferWithTimeTask<T, E, OR, OE, S> =
    TimeShiftTask<OE, S, Bundles<T, NoOverlap>, Vec<T>, E, OR>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        Vec<T>,
        E,
        time_shift::Model<Bundles<T, NoOverlap>>,
        ChainDisposal<SD, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTime<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    /// The timer's thread emits the buffers, the source's the last one, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTime<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // A `BufferWithTimeAndShift` whose buffers start as the one before ends: a single one is
        // ever open, so no item is cloned (decision 0014).
        time_shift::subscribe(
            self.source,
            self.time_span,
            self.time_span,
            self.scheduler,
            Bundles::with_room(NoOverlap, 0),
            observer,
        )
        .map_inner_into()
    }
}
