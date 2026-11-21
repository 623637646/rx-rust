//! The machinery shared by [`Debounce`](super::super::debounce::Debounce),
//! [`Audit`](super::super::audit::Audit) and
//! [`ThrottleLatest`](super::super::throttle_latest::ThrottleLatest), the operators that limit
//! the rate of their source with spans: an item opens a span of one `time_span`, whose end, its
//! deadline, a timer waits for, and the span holds at most one value, the pending one, which its
//! end emits.
//!
//! What sets the three apart — whether an item is emitted at once or held, which deadline it
//! sets, what the end of a span does — is each operator's [`SpanState`], kept under the lock of
//! the context. This module runs it: it takes the lock for each item, step of the timer and
//! completion, delivers what the state hands back, drops a replaced value outside the lock, and
//! wakes or finishes the timer, one wakeable task for the whole subscription.
//!
//! A completion emits the pending value, then completes; an error drops it.

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::{UpdateOutcome, Updated},
        subscribe_with_context::{
            SubscriptionContext, TwoStepsDisposal, subscribe_with_context_two_steps,
        },
    },
};
use educe::Educe;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

/// The state of an operator built on this module, and what it does with each event. It runs
/// under the lock of the context, so it must not call anything that can reach the operator again:
/// what it emits or replaces is handed back, and delivered or dropped outside the lock.
pub(in crate::operators::filtering) trait SpanState<T>:
    Default
{
    /// An item from the source. `now` reads the clock, under the lock (decision 0008) and once at
    /// most: call it only on a path that needs the time.
    fn on_item(
        &mut self,
        value: T,
        now: impl FnOnce() -> Instant,
        time_span: Duration,
    ) -> OnItem<T>;

    /// A step of the timer, at `now`: woken by [`OnItem::Hold`], or at the instant of the last
    /// [`OnTimer::WaitUntil`]. A `Debounce` deadline that moved since wakes it early, so a step
    /// finds a value that is not yet due.
    fn on_timer(&mut self, now: Instant, time_span: Duration) -> OnTimer<T>;

    /// The completion of the source: the pending value, which it emits before it completes.
    fn on_completed(&mut self) -> Option<T>;
}

/// What an item does.
#[derive(Educe)]
#[educe(Debug, PartialEq)]
pub(in crate::operators::filtering) enum OnItem<T> {
    /// Emitted at once.
    Emit(T),
    /// The pending value, until a deadline.
    Hold {
        /// Whether to wake the timer, so that it waits for the item's deadline: true when it waits
        /// for none yet, unless that deadline is too far out for an [`Instant`] to represent, and
        /// the item then waits for the completion.
        wake_timer: bool,
        /// The value the item replaced, dropped outside the lock.
        replaced: Option<T>,
    },
}

/// What a step of the timer does.
#[derive(Educe)]
#[educe(Debug, PartialEq)]
pub(in crate::operators::filtering) enum OnTimer<T> {
    /// Emits the pending value, now due, then waits for the next item: nothing else is pending.
    Emit(T),
    /// Waits for the deadline of the pending value, or for an item that moves it.
    WaitUntil(Instant),
    /// No pending value comes due: waits for an item. The timer finishes instead once the
    /// source has let go of its observer, so that nothing more can come (decision 0004).
    WaitForItem,
}

/// The span of `Debounce` and `Audit`, trailing-edge operators: the item that opens it is its
/// pending value, and its end emits the pending value and closes it.
pub(in crate::operators::filtering) struct TrailingSpan<T> {
    /// The pending value.
    pub(in crate::operators::filtering) value: T,
    /// When the span ends, or `None` when that is too far out for an [`Instant`] to represent: the
    /// value then waits for the completion. The clock only moves forward, so a later deadline is
    /// `None` too.
    pub(in crate::operators::filtering) deadline: Option<Instant>,
}

impl<T> TrailingSpan<T> {
    /// Opens in `slot`, where no span is open, a span that ends one `time_span` after `now`, with
    /// `value` pending.
    pub(in crate::operators::filtering) fn open(
        slot: &mut Option<Self>,
        value: T,
        now: Instant,
        time_span: Duration,
    ) -> OnItem<T> {
        let deadline = now.checked_add(time_span);
        *slot = Some(Self { value, deadline });
        OnItem::Hold {
            wake_timer: deadline.is_some(),
            replaced: None,
        }
    }

    /// The step of the timer at `now`: the end of the span open in `slot` emits its value and
    /// closes it.
    pub(in crate::operators::filtering) fn on_timer(
        slot: &mut Option<Self>,
        now: Instant,
    ) -> OnTimer<T> {
        let ended = |span: &mut Self| span.deadline.is_some_and(|deadline| deadline <= now);
        if let Some(span) = slot.take_if(ended) {
            return OnTimer::Emit(span.value);
        }
        match slot.as_ref().and_then(|span| span.deadline) {
            Some(deadline) => OnTimer::WaitUntil(deadline),
            None => OnTimer::WaitForItem,
        }
    }
}

/// The timer, one wakeable task for the whole subscription, which the source's observer wakes.
///
/// The task holds the context strongly, so that a pending value is still emitted after the source
/// dropped its observer; it finishes once no pending value comes due and the source sends nothing
/// more. A disposal still releases the observer at once: the source drops its own handle
/// as it is disposed, and a handle dropped once the context has stopped releases the observer (see
/// `SerializedDelivery`).
pub(in crate::operators::filtering) type SpanTask<OE, S, OR, St> = WakeableContext<
    Timer<
        JoinedSchedulerModeOf<OE, S>,
        <OE as ObservableTypes>::Item,
        <OE as ObservableTypes>::Error,
        OR,
        St,
        ChainSchedulerDisposalOf<S, OE>,
    >,
    JoinedSchedulerModeOf<OE, S>,
>;

/// The observer the operator over `OE` and `S`, with the state `St`, gives its source.
pub(in crate::operators::filtering) type SpanObserverOf<OE, S, OR, St> = SpanObserver<
    JoinedSchedulerModeOf<OE, S>,
    <OE as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    OR,
    St,
    ChainSchedulerDisposalOf<S, OE>,
    <S as SchedulerTypes>::Clock,
>;

/// The disposal of a subscription, the timer's `SD` first, then the source's `D`. Each operator
/// names it with its own state through `delegate_disposal!`.
pub(in crate::operators::filtering) type SpanDisposal<M, T, E, St, SD, D> =
    TwoStepsDisposal<M, T, E, St, D, SD>;

/// The disposal of the operator over `OE` and `S`, with the state `St`.
type SpanDisposalOf<OE, S, St> = SpanDisposal<
    JoinedSchedulerModeOf<OE, S>,
    <OE as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    St,
    <S as SchedulerTypes>::Disposal,
    <OE as ObservableTypes>::Disposal,
>;

/// Subscribes `source`, then starts the timer, unless the source ended the stream as it was
/// subscribed. `St` is the operator's state.
pub(in crate::operators::filtering) fn subscribe<T, E, OE, S, OR, St>(
    source: OE,
    time_span: Duration,
    scheduler: S,
    observer: OR,
) -> DisposeOnDrop<SpanDisposalOf<OE, S, St>>
where
    OR: Observer<T, E>,
    OE: Observable<SpanObserverOf<OE, S, OR, St>, Item = T, Error = E>,
    S: Scheduler<SpanTask<OE, S, OR, St>>,
    St: SpanState<T>,
{
    // The task is created with the source's observer, which wakes it, and started by the second
    // step, which takes it over from the first.
    let timer = Cell::new(None);
    subscribe_with_context_two_steps(
        observer,
        St::default(),
        |context| {
            let (task, waker) = Task::wakeable(
                Timer {
                    context: context.clone(),
                    time_span,
                },
                step_timer,
            );
            timer.set(Some(task));
            Some(source.subscribe(SpanObserver {
                context: context.clone(),
                time_span,
                clock: scheduler.clock(),
                waker,
            }))
        },
        // Skipped when the source ended the stream as it was subscribed — a completion emits the
        // pending value and ends the stream at once — so there is nothing left to time. The
        // context owns only the task's disposal, never the task, which its runtime owns: holding
        // the context strongly forms no cycle. Stopping the context disposes the timer, which
        // cancels the task.
        |_| timer.take().map(|task| scheduler.run_task(task, None)),
    )
}

/// The state of the timer: the context, and the time span, for a state that opens a span as the
/// timer emits (`ThrottleLatest`).
pub(in crate::operators::filtering) struct Timer<M: ThreadMode, T, E, OR, St, D: Disposable> {
    context: SubscriptionContext<M, T, E, OR, St, D>,
    time_span: Duration,
}

/// One step of the timer: what [`SpanState::on_timer`] answers, delivered. `source_closed` is the
/// `closed` of [`Task::wakeable`]: the source let go of its observer, so it sends nothing more.
fn step_timer<M, T, E, OR, St, D>(
    timer: &mut Timer<M, T, E, OR, St, D>,
    source_closed: bool,
    now: Instant,
) -> Next
where
    M: ThreadMode,
    OR: Observer<T, E>,
    St: SpanState<T>,
    D: Disposable,
{
    // With nothing left to time, and the source sending nothing more, the observer is released
    // without a termination (decision 0004).
    let wait_for_item = if source_closed {
        Next::Finished
    } else {
        Next::Pending
    };
    let time_span = timer.time_span;
    timer
        .context
        .update(|state| match state.on_timer(now, time_span) {
            OnTimer::Emit(value) => UpdateOutcome::new(wait_for_item).with_next_event(value),
            OnTimer::WaitUntil(deadline) => {
                UpdateOutcome::new(Next::PendingUntil(deadline)).without_events()
            }
            OnTimer::WaitForItem => UpdateOutcome::new(wait_for_item).without_events(),
        })
        .result()
        .unwrap_or(Next::Finished)
}

/// The observer an operator built on this module gives its source. It holds the scheduler's clock
/// rather than the scheduler, which may be `Local` while the source sends from another thread, and
/// wakes the timer rather than scheduling it.
#[doc(hidden)]
pub struct SpanObserver<M: ThreadMode, T, E, OR, St, D: Disposable, C> {
    context: SubscriptionContext<M, T, E, OR, St, D>,
    time_span: Duration,
    clock: C,
    waker: TaskWaker<M>,
}

impl<M, T, E, OR, St, D, C> Observer<T, E> for SpanObserver<M, T, E, OR, St, D, C>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    St: SpanState<T>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        let updated = self.context.update(|state| {
            match state.on_item(value, || self.clock.now(), self.time_span) {
                OnItem::Emit(value) => UpdateOutcome::new(false)
                    .without_drop_outside()
                    .with_next_event(value),
                OnItem::Hold {
                    wake_timer,
                    replaced,
                } => UpdateOutcome::new(wake_timer)
                    .with_drop_outside(replaced)
                    .without_events(),
            }
        });
        if let Updated::Ran { result: true, .. } = updated {
            self.waker.wake();
        }
        updated.flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update(|state| match state.on_completed() {
                    Some(value) => {
                        UpdateOutcome::empty().with_next_and_termination_events(value, completion)
                    }
                    None => UpdateOutcome::empty().with_termination_event(completion),
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}

#[cfg(test)]
pub(in crate::operators::filtering) mod testing {
    //! What the unit tests of the states share.

    use super::{OnItem, SpanState};
    use std::time::{Duration, Instant};

    pub(in crate::operators::filtering) const DURATION_1_MS: Duration = Duration::from_millis(1);
    pub(in crate::operators::filtering) const DURATION_100_MS: Duration =
        Duration::from_millis(100);

    /// An item at `now`.
    pub(in crate::operators::filtering) fn item<T>(
        state: &mut impl SpanState<T>,
        value: T,
        now: Instant,
        time_span: Duration,
    ) -> OnItem<T> {
        state.on_item(value, move || now, time_span)
    }

    /// An item that must not read the clock.
    pub(in crate::operators::filtering) fn item_without_clock<T>(
        state: &mut impl SpanState<T>,
        value: T,
        time_span: Duration,
    ) -> OnItem<T> {
        state.on_item(
            value,
            || -> Instant { panic!("the item read the clock") },
            time_span,
        )
    }
}
