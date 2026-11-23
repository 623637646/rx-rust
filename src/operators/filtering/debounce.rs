//! The [`Debounce`] operator, behind
//! [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce).

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

/// Emits an item from the source observable only after a particular time span has passed without
/// another source emission.
/// See <https://reactivex.io/documentation/operators/debounce.html>
///
/// A `time_span` too long for an [`Instant`] to represent never passes: an
/// item is emitted only by the completion, which emits the last one.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::filtering::debounce::Debounce,
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
/// let _subscription = Debounce::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// time.advance_by(Duration::from_millis(4));
/// let _ = source.on_next(2); // Restarts the quiet period.
/// time.advance_by(Duration::from_millis(4));
/// assert!(values.with_ref(Vec::is_empty));
/// time.advance_by(Duration::from_millis(1));
/// assert_eq!(values.clone_value(), [2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debounce<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Debounce<OE, S> {
    /// Creates a [`Debounce`] over `source`;
    /// [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce) is the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for Debounce<OE, S>
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
    SpanDisposal<M, T, E, DebounceState<T>, SD, D>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S, OR> Observable<OR> for Debounce<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<SpanObserverOf<OE, S, OR, DebounceState<T>>, Item = T, Error = E>,
    S: Scheduler<SpanTask<OE, S, OR, DebounceState<T>>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        span::subscribe(self.source, self.time_span, self.scheduler, observer).map_inner_into()
    }
}

/// The state of a [`Debounce`]: the span open, if any. The items that follow the one that opened it
/// replace its pending value, and move its end one time span after themselves.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Default(bound()))]
pub struct DebounceState<T>(Option<TrailingSpan<T>>);

impl<T> SpanState<T> for DebounceState<T> {
    fn on_item(
        &mut self,
        value: T,
        now: impl FnOnce() -> Instant,
        time_span: Duration,
    ) -> OnItem<T> {
        match &mut self.0 {
            // The item replaces the pending value, and moves the end of the span. The timer waits for
            // the old end, which is earlier, and then for the new one.
            Some(span) => {
                span.deadline = now().checked_add(time_span);
                OnItem::Hold {
                    wake_timer: false,
                    replaced: Some(std::mem::replace(&mut span.value, value)),
                }
            }
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
            testing::{DURATION_1_MS, DURATION_100_MS, item},
        },
        DebounceState,
    };
    use crate::scheduler::virtual_time::VirtualTime;
    use std::time::Duration;

    #[test]
    fn test_each_item_moves_the_end() {
        let start = VirtualTime::new().now();
        let half = DURATION_100_MS / 2;
        let end = start + half + DURATION_100_MS;
        let mut state = DebounceState::default();

        assert_eq!(
            item(&mut state, 1, start, DURATION_100_MS),
            OnItem::Hold {
                wake_timer: true,
                replaced: None
            }
        );
        assert_eq!(
            item(&mut state, 2, start + half, DURATION_100_MS),
            OnItem::Hold {
                wake_timer: false,
                replaced: Some(1)
            }
        );
        // Woken at the first end, the timer waits on for the moved one.
        assert_eq!(
            state.on_timer(start + DURATION_100_MS, DURATION_100_MS),
            OnTimer::WaitUntil(end)
        );
        assert_eq!(
            state.on_timer(end - DURATION_1_MS, DURATION_100_MS),
            OnTimer::WaitUntil(end)
        );
        assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::Emit(2));
        assert_eq!(state.on_timer(end, DURATION_100_MS), OnTimer::WaitForItem);
        assert_eq!(state.on_completed(), None);
    }

    #[test]
    fn test_time_span_too_long_waits_for_the_completion() {
        let start = VirtualTime::new().now();
        let mut state = DebounceState::default();

        assert_eq!(
            item(&mut state, 1, start, Duration::MAX),
            OnItem::Hold {
                wake_timer: false,
                replaced: None
            }
        );
        assert_eq!(
            item(&mut state, 2, start + DURATION_1_MS, Duration::MAX),
            OnItem::Hold {
                wake_timer: false,
                replaced: Some(1)
            }
        );
        assert_eq!(state.on_timer(start, Duration::MAX), OnTimer::WaitForItem);
        assert_eq!(state.on_completed(), Some(2));
    }
}
