use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    observable::{Observable, Subscription},
    observer::{Event, Observer, Termination},
    scheduler::{RecursionAction, Scheduler},
    utils::{
        pending_events::EventBatch,
        subscribe_with_context::{self, ModelUpdate, SubscriptionContext, subscribe_with_context},
        types::{MarkerType, MaybeSend},
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
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = ObserveOn::new(FromIter::new(vec![1, 2, 3]), handle.clone())
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
pub struct ObserveOn<'or, OE, S> {
    source: OE,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> ObserveOn<'or, OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self {
            source,
            scheduler,
            _marker: Default::default(),
        }
    }
}

impl<'or, T, E, OE, S> Observable<'static, T, E> for ObserveOn<'or, OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'or, T, E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = subscribe_with_context::Disposal<'or, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let model = Model::<T, E, S::D> {
            values: Vec::new(),
            termination: None,
            task: Task::Stopped,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(ObserveOnObserver {
                context,
                scheduler: self.scheduler,
            })
        })
    }
}

/// Events waiting to be observed and the scheduler task that delivers them.
struct Model<T, E, D: Disposable> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
    task: Task<D>,
}

/// Keeps at most one recursive scheduler task alive while events are waiting.
///
/// `Running(None)` covers schedulers that can execute the task before returning its disposal.
enum Task<D: Disposable> {
    Stopped,
    Running(Option<BoundDropDisposal<D>>),
}

struct ObserveOnObserver<T, E, OR, S: Scheduler> {
    context: SubscriptionContext<T, E, OR, Model<T, E, S::D>>,
    scheduler: S,
}

impl<T, E, OR, S: Scheduler> ObserveOnObserver<T, E, OR, S> {
    /// Queues `event` for the observing scheduler, starting the delivering task if it is stopped.
    ///
    /// Only one task exists at a time. `Observer` serializes its callers — `on_next` takes
    /// `&mut self` and `on_termination` takes `self` — so a task cannot be started here while
    /// another call is between starting a task and storing its disposal below.
    fn queue_event(&self, event: Event<T, E>)
    where
        T: MaybeSend + 'static,
        E: MaybeSend + 'static,
        OR: Observer<T, E> + MaybeSend + 'static,
        S: Scheduler + Clone + MaybeSend + 'static,
    {
        let task_setup = self.context.try_update_model(|model| {
            match event {
                Event::Next(value) => model.values.push(value),
                Event::Termination(termination) => model.termination = Some(termination),
            }
            let start_task = matches!(model.task, Task::Stopped);
            if start_task {
                model.task = Task::Running(None);
            }
            ModelUpdate::new(start_task)
        });
        let Ok(true) = task_setup else {
            return;
        };

        // The context owns this task through the model, so the task only holds a weak reference
        // back: a strong one would form a cycle and leak the subscription.
        let weak_context = self.context.downgrade();
        let task = self.scheduler.schedule_recursively(
            move |_| {
                let Some(context) = weak_context.upgrade() else {
                    return RecursionAction::Stop;
                };
                context
                    .try_update_model(|model| {
                        let termination = model.termination.take();
                        let values = std::mem::take(&mut model.values);
                        let (action, events, discarded_values) = match termination {
                            // Nothing left to deliver. An empty batch is a no-op for the context,
                            // and every branch must produce one so their types agree.
                            None if values.is_empty() => (
                                RecursionAction::Stop,
                                EventBatch::NextBatch(Vec::new()),
                                None,
                            ),
                            // Recur instead of stopping: values arriving while this batch is
                            // delivered are pushed onto the model, and only another pass takes
                            // them. They cannot start a task of their own, because this one is
                            // still `Running` until a pass finds the model empty.
                            None => (
                                RecursionAction::ContinueImmediately,
                                EventBatch::NextBatch(values),
                                None,
                            ),
                            Some(completion @ Termination::Completed) => (
                                RecursionAction::Stop,
                                EventBatch::NextBatchAndTermination(values, completion),
                                None,
                            ),
                            // An error preempts the values buffered before it, unlike a completion.
                            Some(error @ Termination::Error(_)) => (
                                RecursionAction::Stop,
                                EventBatch::Termination(error),
                                Some(values),
                            ),
                        };
                        let finished_task = matches!(action, RecursionAction::Stop)
                            .then(|| std::mem::replace(&mut model.task, Task::Stopped));
                        ModelUpdate::new(action)
                            .with_events(events)
                            .with_drop_outside((finished_task, discarded_values))
                    })
                    .unwrap_or(RecursionAction::Stop)
            },
            None,
        );

        let mut task = Some(task);
        let _ = self.context.try_update_model(move |model| {
            if let Task::Running(slot) = &mut model.task
                && slot.is_none()
            {
                *slot = task.take();
            }
            // If the task already stopped, dispose the returned handle outside the lock.
            ModelUpdate::empty().with_drop_outside(task)
        });
    }
}

impl<T, E, OR, S> Observer<T, E> for ObserveOnObserver<T, E, OR, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        self.queue_event(Event::Next(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.queue_event(Event::Termination(termination));
    }
}
