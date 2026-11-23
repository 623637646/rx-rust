//! The [`Audit`] operator, behind
//! [`ObservableExt::audit`](crate::observable::ObservableExt::audit).

use super::internal::span::{
    self, OnItem, OnTimer, SpanDisposal, SpanObserverOf, SpanState, SpanTask, TrailingSpan,
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    scheduler::{Scheduler, SchedulerTypes},
    thread_mode::ThreadMode,
};
use educe::Educe;
use std::time::{Duration, Instant};

#[doc(hidden)]
pub use super::internal::span::SpanObserver;

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
/// long for an [`Instant`] to represent never passes: an item is emitted only
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

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    SpanDisposal<M, T, E, AuditState<T>, SD, D>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S, OR> Observable<OR> for Audit<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<SpanObserverOf<OE, S, OR, AuditState<T>>, Item = T, Error = E>,
    S: Scheduler<SpanTask<OE, S, OR, AuditState<T>>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        span::subscribe(self.source, self.time_span, self.scheduler, observer).map_inner_into()
    }
}

/// The state of an [`Audit`]: the span open, if any. The items that follow the one that opened it
/// replace its pending value, and keep its end.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Default(bound()))]
pub struct AuditState<T>(Option<TrailingSpan<T>>);

impl<T> SpanState<T> for AuditState<T> {
    fn on_item(
        &mut self,
        value: T,
        now: impl FnOnce() -> Instant,
        time_span: Duration,
    ) -> OnItem<T> {
        match &mut self.0 {
            // The item replaces the pending value, and keeps the end of the span: it needs no time.
            Some(span) => OnItem::Hold {
                wake_timer: false,
                replaced: Some(std::mem::replace(&mut span.value, value)),
            },
            None => TrailingSpan::open(&mut self.0, value, now(), time_span),
        }
    }

    fn on_timer(&mut self, now: Instant, _: Duration) -> OnTimer<T> {
        TrailingSpan::on_timer(&mut self.0, now)
    }

    fn on_completed(&mut self) -> Option<T> {
        self.0.take().map(|span| span.value)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::internal::span::{
            OnItem, OnTimer, SpanState,
            testing::{DURATION_1_MS, DURATION_100_MS, item, item_without_clock},
        },
        AuditState,
    };
    use crate::scheduler::virtual_time::VirtualTime;
    use std::time::Duration;

    #[test]
    fn test_holds_the_last_item_until_the_span_ends() {
        let start = VirtualTime::new().now();
        let end = start + DURATION_100_MS;
        let mut state = AuditState::default();

        assert_eq!(
            item(&mut state, 1, start, DURATION_100_MS),
            OnItem::Hold {
                wake_timer: true,
                replaced: None
            }
        );
        // Keeps the end of the span, so it does not read the clock.
        assert_eq!(
            item_without_clock(&mut state, 2, DURATION_100_MS),
            OnItem::Hold {
                wake_timer: false,
                replaced: Some(1)
            }
        );
        assert_eq!(
            state.on_timer(end - DURATION_1_MS, DURATION_100_MS),
            OnTimer::WaitUntil(end)
        );
        assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::Emit(2));

        // The emission closed the span: the next item opens another.
        assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::WaitForItem);
        assert_eq!(
            item(&mut state, 3, end + DURATION_1_MS, DURATION_100_MS),
            OnItem::Hold {
                wake_timer: true,
                replaced: None
            }
        );
        assert_eq!(state.on_completed(), Some(3));
    }

    #[test]
    fn test_time_span_too_long_waits_for_the_completion() {
        let start = VirtualTime::new().now();
        let mut state = AuditState::default();

        assert_eq!(
            item(&mut state, 1, start, Duration::MAX),
            OnItem::Hold {
                wake_timer: false,
                replaced: None
            }
        );
        assert_eq!(state.on_timer(start, Duration::MAX), OnTimer::WaitForItem);
        assert_eq!(state.on_completed(), Some(1));
        assert_eq!(state.on_completed(), None);
    }
}
