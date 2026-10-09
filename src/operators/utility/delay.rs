//! The [`Delay`] operator, behind
//! [`ObservableExt::delay`](crate::observable::ObservableExt::delay).

use crate::delegate_disposal;
use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::observer::EventBatch;
use crate::scheduler::{RecursiveContext, SchedulerTypes, Task};
use crate::thread_mode::{Joined, ThreadMode};
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::utils::subscription_slot::SubscriptionSlot;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Scheduler, TaskState},
};
use educe::Educe;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Shifts the emissions from an Observable forward in time by a specified duration.
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
///         observer::Termination,
///         operators::{
///             creating::from_iter::FromIter,
///             utility::delay::Delay,
///         },
///     };
///     use std::{
///         sync::{Arc, Mutex},
///         time::Duration,
///     };
///     use tokio::time::sleep;
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = Delay::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
///         scheduler.clone(),
///     )
///     .subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
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

/// The thread mode of a [`Delay`]: the timer's thread delivers the values, the source's the
/// errors.
type DelayMode<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The context of a [`Delay`] subscription.
type DelayContext<M, T, E, OR, S, D> =
    SubscriptionContext<M, T, E, OR, Model<T, <S as SchedulerTypes>::Disposal>, D>;

/// The task of a [`Delay`] timer. It holds the context strongly, so that the values already delayed
/// are delivered after the source has let go of its observer, terminated or not. A disposal still
/// releases the observer at once: the source drops its own handle as it is disposed, and a handle
/// dropped once the context has stopped releases the observer.
type DelayTask<M, T, E, OR, S, D> = RecursiveContext<DelayContext<M, T, E, OR, S, D>>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T, SD>, D>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Delay<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = DelayMode<OE, S>;
    type Disposal = Disposal<DelayMode<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Delay<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<
            DelayObserver<DelayMode<OE, S>, T, E, OR, S, <OE as ObservableTypes>::Disposal>,
            Item = T,
            Error = E,
        >,
    S: Scheduler<DelayTask<DelayMode<OE, S>, T, E, OR, S, <OE as ObservableTypes>::Disposal>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let model = Model::<T, S::Disposal> {
            values: VecDeque::new(),
            completion: None,
            timer: SubscriptionSlot::Idle,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(DelayObserver {
                context,
                delay: self.delay,
                scheduler: self.scheduler,
            })
        })
        .map_into()
    }
}

/// The state of a [`Delay`] subscription.
struct Model<T, D: Disposable> {
    /// The values with the instant at which each of them is due, in ascending order.
    values: VecDeque<(Instant, T)>,
    /// The instant at which the completion is due, once the source has completed.
    completion: Option<Instant>,
    /// Keeps at most one recursive scheduler task alive while events are waiting. The slot is
    /// reserved while the task is being scheduled, which covers schedulers that can execute a
    /// zero-delay task before returning its disposal.
    timer: SubscriptionSlot<BoundDropDisposal<D>>,
}

impl<T, D: Disposable> Model<T, D> {
    /// Returns the instant at which the next event is due.
    fn next_deadline(&self) -> Option<Instant> {
        self.values
            .front()
            .map(|(deadline, _)| *deadline)
            .or(self.completion)
    }
}

pub struct DelayObserver<M, T, E, OR, S, D>
where
    D: Disposable,
    M: ThreadMode,
    S: SchedulerTypes,
{
    context: DelayContext<M, T, E, OR, S, D>,
    delay: Duration,
    scheduler: S,
}

impl<M, T, E, OR, S, D> DelayObserver<M, T, E, OR, S, D>
where
    D: Disposable,
    OR: Observer<T, E>,
    M: ThreadMode,
    S: SchedulerTypes,
{
    /// Queues `value`, or the completion when it is `None`, and starts the timer if needed.
    ///
    /// Returns [`Flow::Stop`] once the context has stopped: the event was then dropped, and so
    /// would every later one be. What downstream itself answers is only known once the timer
    /// fires, so this is otherwise [`Flow::Continue`].
    // The scheduler bound is on the method rather than the impl: it names the private model, which
    // the bounds of an impl on a public type may not.
    fn queue_event(&self, value: Option<T>) -> Flow
    where
        S: Scheduler<DelayTask<M, T, E, OR, S, D>>,
    {
        let Some(deadline) = self.scheduler.now().checked_add(self.delay) else {
            // The event never comes due, and neither does any later one, since the clock only
            // moves forward: there is nothing to queue, and the values already queued keep their
            // timer. The context still answers, so that a stopped one says `Stop` as it does for
            // an event in range; the event is dropped here, outside its lock.
            return self
                .context
                .update_flow(|_| UpdateOutcome::empty().without_events());
        };
        let timer_setup = self.context.update(|model| {
            match value {
                Some(value) => model.values.push_back((deadline, value)),
                None => model.completion = Some(deadline),
            }
            let start_timer = model.timer.reserve_if_idle();
            UpdateOutcome::new(start_timer.then_some(deadline))
        });
        let deadline = match timer_setup {
            Ok(Some(deadline)) => deadline,
            // A build is in flight, so this event is left in the model for whoever finishes that
            // handoff: either the running timer finds it when it fires, or — when the timer has
            // already stopped — the `fill` below that hands its handle back does.
            Ok(None) => return Flow::Continue,
            Err(DeliveryStopped) => return Flow::Stop,
        };

        // The model holds only the task's disposal, never the task, which its runtime owns: holding
        // the context strongly forms no cycle. Stopping the context drops the model, which cancels
        // the task.
        let task = Task::recursive(self.context.clone(), |context, _, now| {
            context
                .update(|model| {
                    // The deadlines are ascending, so one binary search splits the queue into
                    // the due values and the ones that keep waiting.
                    let due = model
                        .values
                        .partition_point(|(deadline, _)| *deadline <= now);
                    let values: Vec<T> =
                        model.values.drain(..due).map(|(_, value)| value).collect();

                    // The completion is queued last, so it is due only once no value is left
                    // to deliver before it and its own deadline has passed. A value that
                    // still waits holds the completion back, which keeps the order right.
                    if model.values.is_empty()
                        && model.completion.is_some_and(|deadline| deadline <= now)
                    {
                        return UpdateOutcome::new(TaskState::Finished)
                            .with_events(EventBatch::NextBatchAndTermination(
                                values,
                                Termination::Completed,
                            ))
                            .with_drop_outside(model.timer.release());
                    }

                    match model.next_deadline() {
                        Some(deadline) => UpdateOutcome::new(TaskState::SleepUntil(deadline))
                            .with_events(EventBatch::NextBatch(values))
                            .without_drop_outside(),
                        // Nothing is waiting anymore: stop the timer until the next event.
                        None => UpdateOutcome::new(TaskState::Finished)
                            .with_events(EventBatch::NextBatch(values))
                            .with_drop_outside(model.timer.release()),
                    }
                })
                .unwrap_or(TaskState::Finished)
        });
        let disposal = self.scheduler.run_task(
            task,
            Some(deadline.saturating_duration_since(self.scheduler.now())),
        );

        // A zero delay can fire the timer before this runs, and that firing can end the stream:
        // the flow of this update is what reports it.
        self.context.update_flow(move |model| {
            // If the timer already stopped (possible for a zero delay), `fill` gives the handle
            // back to dispose outside the lock.
            UpdateOutcome::empty().with_drop_outside(model.timer.fill(disposal))
        })
    }
}

impl<M, T, E, OR, S, D> Observer<T, E> for DelayObserver<M, T, E, OR, S, D>
where
    D: Disposable,
    OR: Observer<T, E>,
    M: ThreadMode,
    S: Scheduler<DelayTask<M, T, E, OR, S, D>>,
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
            // values that are still waiting along with the timer.
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
