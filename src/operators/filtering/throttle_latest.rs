//! Emits an item at once, then the latest one at the end of each time span: a leading- and
//! trailing-edge throttle.
//!
//! The [`ThrottleLatest`] operator, behind
//! [`ObservableExt::throttle_latest`](crate::observable::ObservableExt::throttle_latest).

use super::internal::span::{
    self, OnItem, OnTimer, SpanDisposal, SpanObserverOf, SpanState, SpanTask,
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

/// Emits an item at once, then, while items keep coming, the most recent one at the end of each
/// time span.
/// See <https://reactivex.io/documentation/operators/sample.html>; RxJava calls it
/// `throttleLatest`.
///
/// A leading- and trailing-edge throttle. An item that arrives while no span is open is emitted
/// at once and opens a span; the items that follow within it only replace the one held, and the
/// span's end emits the last of them and opens the next span. A span that ends with nothing held
/// closes, and the next item is emitted at once again. However busy the source is, the items are
/// thus chosen at least `time_span` apart — but for the one a completion emits, at once.
///
/// That spaces when the items are handed to the observer, not when its `on_next` runs: an observer
/// still busy with an item gets the next one as soon as it returns, so its calls may come closer
/// together. This operator does not pace the delivery itself.
///
/// Where [`Throttle`](crate::operators::filtering::throttle::Throttle) emits the first item of a
/// span and drops the rest, this keeps the last, so the final state of a burst is never lost;
/// where [`Audit`](crate::operators::filtering::audit::Audit) holds even the first item until the
/// span's end, this emits it without delay.
///
/// A completion emits the item being held, then completes; an error drops it. A `time_span` too
/// long for an [`Instant`] to represent never passes: the first item is
/// emitted at once, and of the items after it only the last, by the completion.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::filtering::throttle_latest::ThrottleLatest,
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
/// let _subscription = ThrottleLatest::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1); // Emitted at once, and opens a span.
/// assert_eq!(values.clone_value(), [1]);
/// time.advance_by(Duration::from_millis(1));
/// let _ = source.on_next(2); // Held.
/// let _ = source.on_next(3); // Replaces the value held.
/// time.advance_by(Duration::from_millis(4));
/// assert_eq!(values.clone_value(), [1, 3]); // The span's end emits it, and opens the next span.
/// time.advance_by(Duration::from_millis(5)); // That span ends with nothing held.
/// let _ = source.on_next(4); // Emitted at once.
/// assert_eq!(values.clone_value(), [1, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ThrottleLatest<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> ThrottleLatest<OE, S> {
    /// Creates a [`ThrottleLatest`] over `source`;
    /// [`ObservableExt::throttle_latest`](crate::observable::ObservableExt::throttle_latest) is
    /// the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for ThrottleLatest<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The source's thread emits the terminations and the items that open a span, the timer's the
    /// items held until a span's end, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    SpanDisposal<M, T, E, ThrottleLatestState<T>, SD, D>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S, OR> Observable<OR> for ThrottleLatest<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<SpanObserverOf<OE, S, OR, ThrottleLatestState<T>>, Item = T, Error = E>,
    S: Scheduler<SpanTask<OE, S, OR, ThrottleLatestState<T>>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        span::subscribe(self.source, self.time_span, self.scheduler, observer).map_inner_into()
    }
}

/// The state of a [`ThrottleLatest`]: the span open, if any.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Default(bound()))]
pub struct ThrottleLatestState<T>(Option<Span<T>>);

/// An open span.
struct Span<T> {
    /// When the span ends, or `None` when that is too far out for an [`Instant`] to represent: it
    /// then lasts until the stream ends.
    deadline: Option<Instant>,
    /// The value the end of the span emits. A span holds none until an item follows the emission
    /// that opened it, and nothing times it until then: it may have ended already, which the next
    /// item tells by the deadline.
    pending: Option<T>,
}

impl<T> Span<T> {
    /// The span an emission at `now` opens: it holds nothing, and ends one `time_span` later.
    fn opened_at(now: Instant, time_span: Duration) -> Self {
        Self {
            deadline: now.checked_add(time_span),
            pending: None,
        }
    }
}

impl<T> SpanState<T> for ThrottleLatestState<T> {
    fn on_item(
        &mut self,
        value: T,
        now: impl FnOnce() -> Instant,
        time_span: Duration,
    ) -> OnItem<T> {
        match &mut self.0 {
            // No span is open: the item is emitted at once, and opens one.
            None => {
                self.0 = Some(Span::opened_at(now(), time_span));
                OnItem::Emit(value)
            }
            // A span with a pending value: the item replaces it, and keeps the end of the span, so
            // it needs no time.
            Some(Span {
                pending: pending @ Some(_),
                ..
            }) => OnItem::Hold {
                wake_timer: false,
                replaced: pending.replace(value),
            },
            // A span that holds nothing, which nothing times: the item tells whether it has ended.
            Some(Span { deadline, pending }) => {
                let now = now();
                if deadline.is_some_and(|deadline| deadline <= now) {
                    // It has: as though no span were open.
                    self.0 = Some(Span::opened_at(now, time_span));
                    return OnItem::Emit(value);
                }
                // It has not: the item waits for its end, which the timer now waits for, unless it
                // never comes.
                *pending = Some(value);
                OnItem::Hold {
                    wake_timer: deadline.is_some(),
                    replaced: None,
                }
            }
        }
    }

    fn on_timer(&mut self, now: Instant, time_span: Duration) -> OnTimer<T> {
        // The timer times only a pending value, of a span that ends.
        let Some(Span {
            deadline: Some(deadline),
            pending: pending @ Some(_),
        }) = &mut self.0
        else {
            // No span is open, the one open holds nothing — the next item tells whether it has
            // ended — or it lasts until the stream ends.
            return OnTimer::WaitForItem;
        };
        let deadline = *deadline;
        match pending.take_if(|_| deadline <= now) {
            // The end of the span emits the pending value, and opens the next span, so that the
            // next emission is one time span later at the earliest.
            Some(value) => {
                self.0 = Some(Span::opened_at(now, time_span));
                OnTimer::Emit(value)
            }
            None => OnTimer::WaitUntil(deadline),
        }
    }

    fn on_completed(&mut self) -> Option<T> {
        self.0.take().and_then(|span| span.pending)
    }
}

#[cfg(test)]
mod tests;
