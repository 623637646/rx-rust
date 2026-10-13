//! The [`ObserveOn`] operator, behind
//! [`ObservableExt::observe_on`](crate::observable::ObservableExt::observe_on).

use crate::delegate_disposal;
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::{
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Event, EventBatch, Flow, Observer, Termination},
    scheduler::{Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
    thread_mode::ThreadMode,
    utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
};
use educe::Educe;
use std::time::Instant;

/// Specifies the `Scheduler` on which an observer will observe this Observable: every event is
/// delivered from a task of that scheduler, in order.
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
///
/// The source is subscribed with an observer that only queues the events and wakes the task of
/// `scheduler`, which is started when the operator is subscribed. So a source that emits from
/// another thread can be observed on a `Local` scheduler, bound to the thread it was made on:
///
/// ```no_run
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::from_future::FromFuture,
///     scheduler::runtime::tokio::{TokioLocalScheduler, TokioScheduler},
/// };
///
/// let _subscription = FromFuture::new(async { 1 }, TokioScheduler::current())
///     .observe_on(TokioLocalScheduler::ambient())
///     .subscribe_with_callback(|_| {}, |_| {});
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

/// The context of an [`ObserveOn`] subscription.
type ObserveOnContext<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T, E>, D>;

/// The state of the [`ObserveOn`] task, one wakeable task for the whole subscription, which the
/// source's observer wakes.
///
/// The task holds the context strongly, so that the events already queued are delivered after the
/// source has let go of its observer, terminated or not; it finishes once nothing is left. A
/// disposal still releases the observer at once: the source drops its own handle as it is
/// disposed, and a handle dropped once the context has stopped releases the observer.
type ObserveOnTask<M, T, E, OR, D> = WakeableContext<ObserveOnContext<M, T, E, OR, D>, M>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T, E>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for ObserveOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// Every event is delivered by the scheduler's task, so the mode is the scheduler's.
    type Mode = S::Mode;
    /// The context is shared between the source's thread and the scheduler's, so its mode is both
    /// joined.
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for ObserveOn<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<
            ObserveOnObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<
        ObserveOnTask<JoinedSchedulerModeOf<OE, S>, T, E, OR, ChainSchedulerDisposalOf<S, OE>>,
    >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model::<T, E> {
            values: Vec::new(),
            termination: None,
        };
        subscribe_with_context(observer, model, |context| {
            let (task, waker) = Task::wakeable(context.clone(), step_task);
            let source_subscription = self.source.subscribe(ObserveOnObserver { context, waker });
            // The context owns only the task's disposal, through its source subscription, never
            // the task, which its runtime owns: holding the context strongly forms no cycle.
            // Stopping the context disposes the task.
            let task = self.scheduler.run_task(task, None);
            source_subscription.preceded_by_wrapped(task)
        })
        .map_inner_into()
    }
}

/// The state of an [`ObserveOn`] subscription.
struct Model<T, E> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
}

/// One step of the task: delivers the events queued since the last one, then waits for the source
/// to send more. `source_closed` is the `closed` of [`Task::wakeable`]: the source let go of its
/// observer, so it sends nothing more.
fn step_task<M, T, E, OR, D>(
    context: &mut ObserveOnContext<M, T, E, OR, D>,
    source_closed: bool,
    _: Instant,
) -> Next
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    context
        .update(|model| {
            let termination = model.termination.take();
            let values = std::mem::take(&mut model.values);
            let (next, events, discarded_values) = match termination {
                // Nothing left to deliver. An empty batch is a no-op for the context, and every
                // branch must produce one so their types agree. Once the source sends nothing
                // more either, the observer is released without a termination (decision 0004).
                None if values.is_empty() => {
                    let next = if source_closed {
                        Next::Finished
                    } else {
                        Next::Pending
                    };
                    (next, EventBatch::NextBatch(Vec::new()), None)
                }
                // Yield rather than park, so that the values arriving while this batch is
                // delivered are taken by the next pass without waiting for a wake.
                None => (Next::Yield, EventBatch::NextBatch(values), None),
                Some(completion @ Termination::Completed) => (
                    Next::Finished,
                    EventBatch::NextBatchAndTermination(values, completion),
                    None,
                ),
                // An error preempts the values buffered before it, unlike a completion.
                Some(error @ Termination::Error(_)) => {
                    (Next::Finished, EventBatch::Termination(error), Some(values))
                }
            };
            UpdateOutcome::new(next)
                .with_events(events)
                .with_drop_outside(discarded_values)
        })
        .unwrap_or(Next::Finished)
}

/// The observer an [`ObserveOn`] gives its source. It queues the events and wakes the task, and
/// holds no scheduler, which may be `Local` while the source sends from another thread.
pub struct ObserveOnObserver<M: ThreadMode, T, E, OR, D: Disposable> {
    context: ObserveOnContext<M, T, E, OR, D>,
    waker: TaskWaker<M>,
}

impl<M, T, E, OR, D> ObserveOnObserver<M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Queues `event` for the observing scheduler, and wakes its task if nothing was queued.
    ///
    /// Returns [`Flow::Stop`] once the context has stopped: the event was then dropped, and so
    /// would every later one be. What downstream itself answers is only known once the task
    /// delivers, so this is otherwise [`Flow::Continue`].
    fn queue_event(&self, event: Event<T, E>) -> Flow {
        let wake = self.context.update(|model| {
            // A task woken for an earlier event that it has not taken yet takes this one too.
            let wake = model.values.is_empty() && model.termination.is_none();
            match event {
                Event::Next(value) => model.values.push(value),
                Event::Termination(termination) => model.termination = Some(termination),
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

impl<M, T, E, OR, D> Observer<T, E> for ObserveOnObserver<M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.queue_event(Event::Next(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        // The termination is the last event, so what the context answers is of no use here.
        let _ = self.queue_event(Event::Termination(termination));
    }
}
