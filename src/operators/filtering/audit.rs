//! The [`Audit`] operator, behind
//! [`ObservableExt::audit`](crate::observable::ObservableExt::audit).

#[doc(hidden)]
pub use super::internal::deadline::DeadlineObserver;
pub use super::internal::deadline::Disposal;
use super::internal::deadline::{self, DeadlineObserverOf, DeadlineTask};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    scheduler::{Scheduler, SchedulerTypes},
};
use educe::Educe;
use std::time::Duration;

/// Emits the most recent item of the source once a time span has passed since the first item that
/// started it, then waits for the next item to start another.
/// See <https://reactivex.io/documentation/operators/sample.html>; RxJS calls it `auditTime`.
/// Unlike `throttleLast` (a periodic `sample`), the span starts on the first item, not on a fixed
/// timer.
///
/// A trailing-edge throttle: the first item starts the span, the items that follow within it only
/// replace the one held, and the span's end emits the last of them. Where
/// [`Debounce`](crate::operators::filtering::debounce::Debounce) restarts the span on every item,
/// and so waits for a pause that a steady stream never makes, this emits once per span however
/// busy the source is; where [`Throttle`](crate::operators::filtering::throttle::Throttle) emits
/// the first item of a span and drops the rest, this emits the last, so the final state of a burst
/// is never lost.
///
/// A completion emits the item being held, then completes; an error drops it. A `time_span` too
/// long for an [`Instant`](std::time::Instant) to represent never passes: an item is emitted only
/// by the completion, which emits the last one.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::filtering::audit::Audit,
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
/// let _subscription = Audit::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1); // Starts the span.
/// time.advance_by(Duration::from_millis(4));
/// let _ = source.on_next(2); // Replaces the value held, without moving the span's end.
/// assert!(values.with_ref(Vec::is_empty));
/// time.advance_by(Duration::from_millis(1));
/// assert_eq!(values.clone_value(), [2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Audit<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Audit<OE, S> {
    /// Creates an [`Audit`] over `source`;
    /// [`ObservableExt::audit`](crate::observable::ObservableExt::audit) is the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for Audit<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The timer's thread emits the values, the source's the terminations, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Audit<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<DeadlineObserverOf<OE, S, OR, false>, Item = T, Error = E>,
    S: Scheduler<DeadlineTask<OE, S, OR>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        deadline::subscribe(self.source, self.time_span, self.scheduler, observer)
    }
}
