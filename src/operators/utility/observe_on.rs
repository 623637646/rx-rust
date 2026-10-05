//! The [`ObserveOn`] operator, behind
//! [`ObservableExt::observe_on`](crate::observable::ObservableExt::observe_on).

use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Event, Flow, Observer, Termination},
    scheduler::{RecursiveContext, Scheduler, SchedulerTypes, Task, TaskState},
    thread_mode::{Joined, ThreadMode},
    utils::{
        pending_events::EventBatch,
        subscribe_with_context::{
            self, PromotableWeakContext, SubscriptionContext, subscribe_with_context,
        },
        subscription_slot::SubscriptionSlot,
    },
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will observe this Observable.
/// See <https://reactivex.io/documentation/operators/observeon.html>
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
///             utility::observe_on::ObserveOn,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = ObserveOn::new(FromIter::new(vec![1, 2, 3]), scheduler.clone())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
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
pub struct ObserveOn<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> ObserveOn<OE, S> {
    /// Creates an [`ObserveOn`] over `source`;
    /// [`ObservableExt::observe_on`](crate::observable::ObservableExt::observe_on) is the fluent
    /// form.
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

/// The thread mode of the state an [`ObserveOn`] shares between its source's thread and the
/// scheduler's.
pub type ObserveOnContextMode<OE, S> =
    Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The context of an [`ObserveOn`] subscription.
pub type ObserveOnContext<M, T, E, OR, S> =
    SubscriptionContext<M, T, E, OR, Model<T, E, <S as SchedulerTypes>::D>>;

/// The task of an [`ObserveOn`]: it holds the context weakly until the source terminates, so
/// that it does not keep the observer alive once the subscription is gone.
pub type ObserveOnTask<M, T, E, OR, S> =
    RecursiveContext<PromotableWeakContext<M, T, E, OR, Model<T, E, <S as SchedulerTypes>::D>>>;

impl<T, E, OE, S> ObservableTypes for ObserveOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// Every event is delivered by the scheduler's task, so the mode is the scheduler's.
    type Mode = S::Mode;
    type D = subscribe_with_context::Disposal<
        ObserveOnContextMode<OE, S>,
        T,
        E,
        Model<T, E, S::D>,
        OE::D,
    >;
}

impl<T, E, OE, S, OR> Observable<OR> for ObserveOn<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<ObserveOnObserver<ObserveOnContextMode<OE, S>, T, E, OR, S>, Item = T, Error = E>,
    S: Scheduler<ObserveOnTask<ObserveOnContextMode<OE, S>, T, E, OR, S>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model::<T, E, S::D> {
            values: Vec::new(),
            termination: None,
            task: SubscriptionSlot::Idle,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(ObserveOnObserver {
                weak_context: PromotableWeakContext::new(&context),
                context,
                scheduler: self.scheduler,
            })
        })
    }
}

/// The state of an [`ObserveOn`] subscription.
pub struct Model<T, E, D: Disposable> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
    /// Keeps at most one recursive scheduler task alive while events are waiting. The slot is
    /// reserved while the task is being scheduled, which covers schedulers that can execute it
    /// before returning its disposal.
    task: SubscriptionSlot<BoundDropDisposal<D>>,
}

pub struct ObserveOnObserver<M, T, E, OR, S>
where
    M: ThreadMode,
    S: SchedulerTypes,
{
    context: ObserveOnContext<M, T, E, OR, S>,
    /// The task's handle on the context, promoted once the source has terminated, with events
    /// still waiting.
    weak_context: PromotableWeakContext<M, T, E, OR, Model<T, E, <S as SchedulerTypes>::D>>,
    scheduler: S,
}

impl<M, T, E, OR, S> ObserveOnObserver<M, T, E, OR, S>
where
    M: ThreadMode,
    S: SchedulerTypes,
{
    /// Queues `event` for the observing scheduler, starting the delivering task if it is stopped.
    ///
    /// Only one task exists at a time. `Observer` serializes its callers — `on_next` takes
    /// `&mut self` and `on_termination` takes `self` — so a task cannot be started here while
    /// another call is between starting a task and storing its disposal below.
    ///
    /// Returns [`Flow::Stop`] once the context has stopped: the event was then dropped, and so
    /// would every later one be. What downstream itself answers is only known once the task
    /// delivers, so this is otherwise [`Flow::Continue`].
    fn queue_event(&self, event: Event<T, E>) -> Flow
    where
        OR: Observer<T, E>,
        S: Scheduler<ObserveOnTask<M, T, E, OR, S>>,
    {
        let task_setup = self.context.update(|model| {
            match event {
                Event::Next(value) => model.values.push(value),
                Event::Termination(termination) => model.termination = Some(termination),
            }
            UpdateOutcome::new(model.task.reserve_if_idle())
        });
        match task_setup {
            Ok(true) => {}
            // A build is in flight, so this event is left in the model for whoever finishes that
            // handoff: either the running task picks it up on its next pass, or — when the task
            // has already stopped — the `fill` below that hands its handle back does.
            Ok(false) => return Flow::Continue,
            Err(DeliveryStopped) => return Flow::Stop,
        }

        // The task holds the context weakly until the source terminates, so that disposing the
        // subscription while the source is active releases the observer at once.
        let task = Task::recursive(self.weak_context.clone(), |weak_context, _| {
            let Some(context) = weak_context.upgrade() else {
                return TaskState::Finished;
            };
            context
                .update(|model| {
                    let termination = model.termination.take();
                    let values = std::mem::take(&mut model.values);
                    let (action, events, discarded_values) = match termination {
                        // Nothing left to deliver. An empty batch is a no-op for the context,
                        // and every branch must produce one so their types agree.
                        None if values.is_empty() => {
                            (TaskState::Finished, EventBatch::NextBatch(Vec::new()), None)
                        }
                        // Recur instead of stopping: values arriving while this batch is
                        // delivered are pushed onto the model, and only another pass takes
                        // them. They cannot start a task of their own, because this one is
                        // still `Running` until a pass finds the model empty.
                        None => (TaskState::Yield, EventBatch::NextBatch(values), None),
                        Some(completion @ Termination::Completed) => (
                            TaskState::Finished,
                            EventBatch::NextBatchAndTermination(values, completion),
                            None,
                        ),
                        // An error preempts the values buffered before it, unlike a completion.
                        Some(error @ Termination::Error(_)) => (
                            TaskState::Finished,
                            EventBatch::Termination(error),
                            Some(values),
                        ),
                    };
                    let finished_task = match action {
                        TaskState::Finished => model.task.release(),
                        _ => None,
                    };
                    UpdateOutcome::new(action)
                        .with_events(events)
                        .with_drop_outside((finished_task, discarded_values))
                })
                .unwrap_or(TaskState::Finished)
        });
        let task = self.scheduler.run_task(task, None);

        // A scheduler that runs the task at once can have delivered, and ended, the stream before
        // this runs: the flow of this update is what reports it.
        self.context.update_flow(move |model| {
            // If the task already stopped, `fill` gives the handle back to dispose outside the
            // lock.
            UpdateOutcome::empty().with_drop_outside(model.task.fill(task))
        })
    }
}

impl<M, T, E, OR, S> Observer<T, E> for ObserveOnObserver<M, T, E, OR, S>
where
    OR: Observer<T, E>,
    M: ThreadMode,
    S: Scheduler<ObserveOnTask<M, T, E, OR, S>>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.queue_event(Event::Next(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        // The termination is the last event, so what the context answers is of no use here.
        let _ = self.queue_event(Event::Termination(termination));
        // The source lets go of this observer now, and so of its handle on the context, while the
        // task still has the termination, and maybe values, to deliver.
        self.weak_context.promote();
    }
}
