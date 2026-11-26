//! Gathers items into `Vec`s over time spans that overlap, or leave gaps.
//!
//! The [`BufferWithTimeAndShift`] operator, behind
//! [`ObservableExt::buffer_with_time_and_shift`](crate::observable::ObservableExt::buffer_with_time_and_shift).

use super::internal::{
    bundle::Bundles,
    overlap::ByClone,
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

/// Gathers the items of an observable into bundles that each last `time_span`, starting a new
/// bundle every `time_shift`, and emits each as a `Vec<T>` as it ends, empty or not.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The first bundle starts at subscription. Later openings are spaced `time_shift` apart on
/// the scheduler's clock. Each item goes to every open bundle. When bundles overlap, the newest
/// takes the original item and earlier bundles receive clones. When `time_shift` exceeds
/// `time_span`, items that arrive in the gaps are dropped. A bundle that ends as the next one
/// starts is emitted first. Use
/// [`BufferWithTime`](super::buffer_with_time::BufferWithTime) when the span and shift are equal
/// and the items lack `Clone`.
///
/// On completion, open bundles with items are emitted oldest first. An error discards all open
/// bundles. A late timer processes overdue openings and closings in their scheduled order.
///
/// Deadlines outside the range of `Instant` never fire. A bundle with an out-of-range closing
/// deadline can still be emitted when the source completes.
///
/// # Panics
///
/// Subscribing panics if `time_span` or `time_shift` is zero.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::buffer_with_time_and_shift::BufferWithTimeAndShift,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let shift = Duration::from_millis(5);
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = values.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription =
///     BufferWithTimeAndShift::new(source.clone(), shift * 2, shift, time.scheduler())
///         .subscribe_with_callback(
///             move |value| values_observer.with_mut(|values| values.push(value)),
///             |_| {},
///         );
///
/// let _ = source.on_next(1);
/// time.advance_by(shift); // The second bundle starts.
/// let _ = source.on_next(2);
/// time.advance_by(shift); // The first ends, and the third starts.
/// assert_eq!(values.clone_value(), [vec![1, 2]]);
/// let _ = source.on_next(3);
/// time.advance_by(shift);
/// assert_eq!(values.clone_value(), [vec![1, 2], vec![2, 3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeAndShift<OE, S> {
    source: OE,
    time_span: Duration,
    time_shift: Duration,
    scheduler: S,
}

impl<OE, S> BufferWithTimeAndShift<OE, S> {
    /// Creates a [`BufferWithTimeAndShift`] over `source`;
    /// [`ObservableExt::buffer_with_time_and_shift`](crate::observable::ObservableExt::buffer_with_time_and_shift)
    /// is the fluent form.
    pub fn new(source: OE, time_span: Duration, time_shift: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            time_shift,
            scheduler,
        }
    }
}

/// The observer a [`BufferWithTimeAndShift`] gives its source.
pub(crate) type BufferWithTimeAndShiftObserver<M, T, E, OR, D> =
    TimeShiftObserver<M, Bundles<T, ByClone>, Vec<T>, E, OR, D>;

/// The task of a [`BufferWithTimeAndShift`] timer. It holds the context strongly, so that the
/// bundles keep being cut after the source has let go of its observer without a termination, until
/// the subscription is disposed.
type BufferWithTimeAndShiftTask<T, E, OR, OE, S> =
    TimeShiftTask<OE, S, Bundles<T, ByClone>, Vec<T>, E, OR>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        Vec<T>,
        E,
        time_shift::Model<Bundles<T, ByClone>>,
        ChainDisposal<SD, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTimeAndShift<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    /// The timer's thread emits the bundles, the source's the last ones, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTimeAndShift<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeAndShiftObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeAndShiftTask<T, E, OR, OE, S>>,
    T: Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        time_shift::subscribe(
            self.source,
            self.time_span,
            self.time_shift,
            self.scheduler,
            Bundles::with_room(ByClone, 0),
            observer,
        )
        .map_inner_into()
    }
}
