//! The state machine of [`Debounce`](super::super::debounce::Debounce) and
//! [`Audit`](super::super::audit::Audit): one pending value, emitted by a timer once its deadline
//! has passed.
//!
//! An item that arrives while nothing is pending becomes the pending value and sets the deadline
//! one `time_span` later. An item that arrives while one is pending replaces it; `MOVES_DEADLINE`,
//! which each operator fixes in the type of the observer its source takes, is what sets the two
//! apart: `Debounce` moves the deadline to one `time_span` after that item, `Audit` keeps the
//! deadline of the item that started the span.
//!
//! A completion emits the pending value, then completes; an error drops it. A deadline too far out
//! for an [`Instant`] to represent never comes: the pending value then waits for the completion.

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::{UpdateOutcome, Updated},
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
    },
};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

/// The context of a subscription.
type DeadlineContext<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T>, D>;

/// The timer, one wakeable task for the whole subscription, which the source's observer wakes.
///
/// The task holds the context strongly, so that a pending value is still emitted after the source
/// dropped its observer; it finishes once nothing is pending and the source sends nothing more. A
/// disposal still releases the observer at once: the source drops its own handle as it is disposed,
/// and a handle dropped once the context has stopped releases the observer (see
/// `SerializedDelivery`).
pub(in crate::operators::filtering) type DeadlineTask<OE, S, OR> = WakeableContext<
    DeadlineContext<
        JoinedSchedulerModeOf<OE, S>,
        <OE as ObservableTypes>::Item,
        <OE as ObservableTypes>::Error,
        OR,
        ChainSchedulerDisposalOf<S, OE>,
    >,
    JoinedSchedulerModeOf<OE, S>,
>;

/// The observer the operator over `OE` and `S` gives its source.
pub(in crate::operators::filtering) type DeadlineObserverOf<OE, S, OR, const MOVES_DEADLINE: bool> =
    DeadlineObserver<
        JoinedSchedulerModeOf<OE, S>,
        <OE as ObservableTypes>::Item,
        <OE as ObservableTypes>::Error,
        OR,
        ChainSchedulerDisposalOf<S, OE>,
        <S as SchedulerTypes>::Clock,
        MOVES_DEADLINE,
    >;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

/// The disposal of the operator over `OE` and `S`.
type DeadlineDisposal<OE, S> = Disposal<
    JoinedSchedulerModeOf<OE, S>,
    <OE as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    <S as SchedulerTypes>::Disposal,
    <OE as ObservableTypes>::Disposal,
>;

/// Subscribes `source`, then starts the timer, unless the source ended the stream as it was
/// subscribed. Each item moves the deadline if `MOVES_DEADLINE`, which the operator fixes in the
/// type of the observer `OE` takes.
pub(in crate::operators::filtering) fn subscribe<T, E, OE, S, OR, const MOVES_DEADLINE: bool>(
    source: OE,
    time_span: Duration,
    scheduler: S,
    observer: OR,
) -> DisposeOnDrop<DeadlineDisposal<OE, S>>
where
    OR: Observer<T, E>,
    OE: Observable<DeadlineObserverOf<OE, S, OR, MOVES_DEADLINE>, Item = T, Error = E>,
    S: Scheduler<DeadlineTask<OE, S, OR>>,
{
    // The task is created with the source's observer, which wakes it, and started by the second
    // step, which takes it over from the first.
    let timer = Cell::new(None);
    subscribe_with_context_two_steps(
        observer,
        Model::Idle,
        |context| {
            let (task, waker) = Task::wakeable(context.clone(), step_timer);
            timer.set(Some(task));
            Some(source.subscribe(DeadlineObserver {
                context: context.clone(),
                time_span,
                clock: scheduler.clock(),
                waker,
            }))
        },
        // Skipped when the source ended the stream as it was subscribed — a completion emits the
        // value being held and ends the stream at once — so there is nothing left to time. The
        // context owns only the task's disposal, never the task, which its runtime owns: holding
        // the context strongly forms no cycle. Stopping the context disposes the timer, which
        // cancels the task.
        |_| timer.take().map(|task| scheduler.run_task(task, None)),
    )
    .map_inner_into()
}

/// The state of a subscription.
pub(in crate::operators::filtering) enum Model<T> {
    Idle,
    Active {
        value: T,
        /// When `value` is emitted, or `None` when that is too far out for an [`Instant`] to
        /// represent: it then waits for the completion. The clock only moves forward, so a later
        /// value's deadline is `None` too.
        deadline: Option<Instant>,
    },
}

/// One step of the timer: emits the pending value once its deadline has passed, then waits for the
/// deadline of the pending value, or for the source to send.
///
/// A `Debounce` value that replaces the pending one only moves the deadline: the timer wakes at the
/// old one, and waits on for the new one. `source_closed` is the `closed` of [`Task::wakeable`]:
/// the source let go of its observer, so it sends nothing more.
fn step_timer<M, T, E, OR, D>(
    context: &mut DeadlineContext<M, T, E, OR, D>,
    source_closed: bool,
    now: Instant,
) -> Next
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    // Once nothing is pending, and the source sends nothing more, the observer is released
    // without a termination (decision 0004).
    let idle = if source_closed {
        Next::Finished
    } else {
        Next::Pending
    };
    context
        .update(|model| match model {
            Model::Active {
                deadline: Some(deadline),
                ..
            } if *deadline <= now => match std::mem::replace(model, Model::Idle) {
                Model::Active { value, .. } => UpdateOutcome::new(idle).with_next_event(value),
                Model::Idle => unreachable!(),
            },
            Model::Active {
                deadline: Some(deadline),
                ..
            } => UpdateOutcome::new(Next::PendingUntil(*deadline)).without_events(),
            // Nothing is pending, or the pending value never comes due: it waits for the
            // completion, which the source's observer emits itself.
            Model::Active { deadline: None, .. } | Model::Idle => {
                UpdateOutcome::new(idle).without_events()
            }
        })
        .result()
        .unwrap_or(Next::Finished)
}

/// The observer a [`Debounce`](super::super::debounce::Debounce) or an
/// [`Audit`](super::super::audit::Audit) gives its source. It holds the scheduler's clock rather
/// than the scheduler, which may be `Local` while the source sends from another thread, and wakes
/// the timer rather than scheduling it.
///
/// `MOVES_DEADLINE` says what an item does while one is pending: if set, it moves the deadline to
/// one `time_span` after itself, as `Debounce` does; if not, it keeps the deadline of the item
/// that started the span, as `Audit` does.
#[doc(hidden)]
pub struct DeadlineObserver<M: ThreadMode, T, E, OR, D: Disposable, C, const MOVES_DEADLINE: bool> {
    context: DeadlineContext<M, T, E, OR, D>,
    time_span: Duration,
    clock: C,
    waker: TaskWaker<M>,
}

impl<M, T, E, OR, D, C, const MOVES_DEADLINE: bool> Observer<T, E>
    for DeadlineObserver<M, T, E, OR, D, C, MOVES_DEADLINE>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        // Read before the update, which needs it only for the item that starts a span or, for
        // `Debounce`, moves it, to keep the lock short.
        let deadline = self.clock.now().checked_add(self.time_span);
        let updated = self.context.update(|model| match model {
            // The timer waits for the source: wake it, unless the value never comes due.
            Model::Idle => {
                *model = Model::Active { value, deadline };
                UpdateOutcome::new(deadline.is_some()).without_drop_outside()
            }
            // The timer already waits for the pending deadline, which `Debounce` only moves later
            // and `Audit` keeps.
            Model::Active {
                value: current_value,
                deadline: current_deadline,
            } => {
                let previous_value = std::mem::replace(current_value, value);
                if MOVES_DEADLINE {
                    *current_deadline = deadline;
                }
                UpdateOutcome::new(false).with_drop_outside(previous_value)
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
                let _ = self
                    .context
                    .update(|model| match std::mem::replace(model, Model::Idle) {
                        Model::Idle => UpdateOutcome::empty().with_termination_event(completion),
                        Model::Active { value, .. } => UpdateOutcome::empty()
                            .with_next_and_termination_events(value, completion),
                    });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
