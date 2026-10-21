//! The [`Delay`] operator, behind
//! [`ObservableExt::delay`](crate::observable::ObservableExt::delay).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{EventBatch, Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::{DeliveryStopped, UpdateOutcome},
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
/// Shifts the emissions from an observable forward in time by a specified duration.
/// See <https://reactivex.io/documentation/operators/delay.html>
///
/// The completion is delayed like the values, but an error is not: it is delivered at once, and
/// the values still waiting are dropped.
///
/// A `delay` too long for an [`Instant`] to represent never ends: the values and the completion
/// never come due, and are dropped as they arrive. An error still comes at once.
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
///         operators::{creating::from_iter::FromIter, utility::delay::Delay},
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use futures::StreamExt;
///     use std::time::Duration;
///
///     let values = Delay::new(
///         FromIter::new([1, 2, 3]), Duration::from_millis(5), TokioScheduler::current(),
///     )
///     .into_stream()
///     .collect::<Vec<_>>()
///     .await;
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
    /// Creates a [`Delay`] over `source`;
    /// [`ObservableExt::delay`](crate::observable::ObservableExt::delay) is the fluent form.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
        }
    }
}

/// The context of a [`Delay`] subscription.
type DelayContext<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T>, D>;

/// The state of the [`Delay`] timer, one wakeable task for the whole subscription, which the
/// source's observer wakes.
///
/// The task holds the context strongly, so that the values already delayed are delivered after the
/// source has let go of its observer, terminated or not; it finishes once nothing is left. A
/// disposal still releases the observer at once: the source drops its own handle as it is
/// disposed, and a handle dropped once the context has stopped releases the observer.
type DelayTask<M, T, E, OR, D> = WakeableContext<DelayContext<M, T, E, OR, D>, M>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Delay<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The timer's thread delivers the values, the source's the errors, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Delay<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<
            DelayObserver<
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
    S: Scheduler<
        DelayTask<JoinedSchedulerModeOf<OE, S>, T, E, OR, ChainSchedulerDisposalOf<S, OE>>,
    >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model::<T> {
            values: VecDeque::new(),
            completion: None,
        };
        subscribe_with_context(observer, model, |context| {
            let (task, waker) = Task::wakeable(context.clone(), step_timer);
            let source_subscription = self.source.subscribe(DelayObserver {
                context,
                delay: self.delay,
                clock: self.scheduler.clock(),
                waker,
            });
            // The context owns only the task's disposal, through its source subscription, never
            // the task, which its runtime owns: holding the context strongly forms no cycle.
            // Stopping the context disposes the timer, which cancels the task.
            let timer = self.scheduler.run_task(task, None);
            source_subscription.preceded_by_wrapped(timer)
        })
        .map_inner_into()
    }
}

/// The state of a [`Delay`] subscription.
struct Model<T> {
    /// The values with the instant at which each of them is due, in ascending order.
    values: VecDeque<(Instant, T)>,
    /// The instant at which the completion is due, once the source has completed.
    completion: Option<Instant>,
}

impl<T> Model<T> {
    /// Returns the instant at which the next event is due.
    fn next_deadline(&self) -> Option<Instant> {
        self.values
            .front()
            .map(|(deadline, _)| *deadline)
            .or(self.completion)
    }
}

/// One step of the timer: delivers the values that are due, and the completion once it is due and
/// no value is left before it, then waits for the next deadline, or for the source to send.
///
/// `source_closed` is the `closed` of [`Task::wakeable`]: the source let go of the observer that
/// holds the [`TaskWaker`] — it terminated, or dropped it — so it sends nothing more.
fn step_timer<M, T, E, OR, D>(
    context: &mut DelayContext<M, T, E, OR, D>,
    source_closed: bool,
    now: Instant,
) -> Next
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    context
        .update(|model| {
            // The deadlines are ascending, so one binary search splits the queue into the due
            // values and the ones that keep waiting.
            let due = model
                .values
                .partition_point(|(deadline, _)| *deadline <= now);
            let values: Vec<T> = model.values.drain(..due).map(|(_, value)| value).collect();

            // The completion is queued last, so it is due only once no value is left to deliver
            // before it and its own deadline has passed. A value that still waits holds the
            // completion back, which keeps the order right.
            if model.values.is_empty() && model.completion.is_some_and(|deadline| deadline <= now) {
                return UpdateOutcome::new(Next::Finished).with_events(
                    EventBatch::NextBatchAndTermination(values, Termination::Completed),
                );
            }

            let next = match model.next_deadline() {
                Some(deadline) => Next::PendingUntil(deadline),
                // Nothing is waiting, and the source sends nothing more: the observer is released
                // without a termination (decision 0004).
                None if source_closed => Next::Finished,
                None => Next::Pending,
            };
            UpdateOutcome::new(next).with_events(EventBatch::NextBatch(values))
        })
        .unwrap_or(Next::Finished)
}

/// The observer a [`Delay`] gives its source. It holds the scheduler's clock rather than the
/// scheduler, which may be `Local` while the source sends from another thread, and wakes the timer
/// rather than scheduling it.
#[doc(hidden)]
pub struct DelayObserver<M: ThreadMode, T, E, OR, D: Disposable, C> {
    context: DelayContext<M, T, E, OR, D>,
    delay: Duration,
    clock: C,
    waker: TaskWaker<M>,
}

impl<M, T, E, OR, D, C> DelayObserver<M, T, E, OR, D, C>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    C: Clock,
{
    /// Queues `value`, or the completion when it is `None`, and wakes the timer if it waits for
    /// nothing.
    ///
    /// Returns [`Flow::Stop`] once the context has stopped: the event was then dropped, and so
    /// would every later one be. What downstream itself answers is only known once the timer
    /// fires, so this is otherwise [`Flow::Continue`].
    fn queue_event(&self, value: Option<T>) -> Flow {
        let Some(deadline) = self.clock.now().checked_add(self.delay) else {
            // The event never comes due, and neither does any later one, since the clock only
            // moves forward: there is nothing to queue, and the values already queued keep their
            // timer. The context still answers, so that a stopped one says `Stop` as it does for
            // an event in range; the event is dropped here, outside its lock.
            return self
                .context
                .update_flow(|_| UpdateOutcome::empty().without_events());
        };
        let wake = self.context.update(|model| {
            // The timer waits for the earliest deadline, which a later event never moves: only an
            // event into an empty queue has a timer to wake.
            let wake = model.next_deadline().is_none();
            match value {
                Some(value) => model.values.push_back((deadline, value)),
                None => model.completion = Some(deadline),
            }
            UpdateOutcome::new(wake).without_events()
        });
        match wake {
            Ok(wake) => {
                if wake {
                    self.waker.wake();
                }
                Flow::Continue
            }
            Err(DeliveryStopped) => Flow::Stop,
        }
    }
}

impl<M, T, E, OR, D, C> Observer<T, E> for DelayObserver<M, T, E, OR, D, C>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.queue_event(Some(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            // The completion is the last event, so what the context answers is of no use here.
            Termination::Completed => {
                let _ = self.queue_event(None);
            }
            // An error is not delayed: it terminates the subscription right away, which drops the
            // values that are still waiting and cancels the timer.
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
