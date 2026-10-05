//! The [`Debounce`] operator, behind
//! [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce).

use crate::delegate_disposal;
use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::thread_mode::{Joined, ThreadMode};
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, WeakSubscriptionContext, subscribe_with_context,
};
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{RecursiveContext, Scheduler, SchedulerTypes, Task, TaskState},
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits an item from the source Observable only after a particular time span has passed without
/// another source emission.
/// See <https://reactivex.io/documentation/operators/debounce.html>
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
///             creating::just::Just,
///             filtering::debounce::Debounce,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = Debounce::new(Just::new(7), Duration::from_millis(5), scheduler.clone())
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
///     assert_eq!(&*values.lock().unwrap(), &[7]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
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

/// The thread mode of a [`Debounce`]: the timer's thread emits the values, the source's the
/// terminations.
type DebounceMode<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The context of a [`Debounce`] subscription.
type DebounceContext<M, T, E, OR, S> =
    SubscriptionContext<M, T, E, OR, Model<T, <S as SchedulerTypes>::D>>;

/// The task of a [`Debounce`] timer: it holds the context weakly, so that it does not keep the
/// observer alive once the subscription is gone.
type DebounceTask<M, T, E, OR, S> =
    RecursiveContext<WeakSubscriptionContext<M, T, E, OR, Model<T, <S as SchedulerTypes>::D>>>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T, SD>, D>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Debounce<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    type Mode = DebounceMode<OE, S>;
    type D = Disposal<DebounceMode<OE, S>, T, E, S::D, OE::D>;
}

impl<T, E, OE, S, OR> Observable<OR> for Debounce<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<DebounceObserver<DebounceMode<OE, S>, T, E, OR, S>, Item = T, Error = E>,
    S: Scheduler<DebounceTask<DebounceMode<OE, S>, T, E, OR, S>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model::<T, S::D>::Idle;
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(DebounceObserver {
                context,
                time_span: self.time_span,
                scheduler: self.scheduler,
            })
        })
        .map_into()
    }
}

/// The state of a [`Debounce`] subscription.
enum Model<T, D: Disposable> {
    Idle,
    Active {
        value: T,
        deadline: Instant,
        timer: Option<BoundDropDisposal<D>>,
    },
}

pub struct DebounceObserver<M, T, E, OR, S>
where
    M: ThreadMode,
    S: SchedulerTypes,
{
    context: DebounceContext<M, T, E, OR, S>,
    time_span: Duration,
    scheduler: S,
}

impl<M, T, E, OR, S> Observer<T, E> for DebounceObserver<M, T, E, OR, S>
where
    OR: Observer<T, E>,
    M: ThreadMode,
    S: Scheduler<DebounceTask<M, T, E, OR, S>>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let timer_setup = self.context.update(|model| {
            let deadline = Instant::now() + self.time_span;
            let (timer_setup, previous_value) = match model {
                Model::Idle => {
                    *model = Model::Active {
                        value,
                        deadline,
                        timer: None,
                    };
                    (Some(deadline), None)
                }
                Model::Active {
                    value: current_value,
                    deadline: current_deadline,
                    ..
                } => {
                    let previous_value = std::mem::replace(current_value, value);
                    *current_deadline = deadline;
                    (None, Some(previous_value))
                }
            };
            UpdateOutcome::new(timer_setup).with_drop_outside(previous_value)
        });
        let deadline = match timer_setup {
            Ok(Some(deadline)) => deadline,
            // The value replaced the pending one, whose timer is still running.
            Ok(None) => return Flow::Continue,
            Err(DeliveryStopped) => return Flow::Stop,
        };

        // The context owns this task through the model, so the task only holds a weak reference
        // back: a strong one would form a cycle and leak the subscription.
        let task = Task::recursive(self.context.downgrade(), |weak_context, _| {
            let Some(context) = weak_context.upgrade() else {
                return TaskState::Finished;
            };
            context
                .update(|model| {
                    let deadline = match model {
                        Model::Idle => {
                            return UpdateOutcome::new(TaskState::Finished)
                                .without_events()
                                .without_drop_outside();
                        }
                        Model::Active { deadline, .. } => *deadline,
                    };
                    if Instant::now() < deadline {
                        return UpdateOutcome::new(TaskState::SleepUntil(deadline))
                            .without_events()
                            .without_drop_outside();
                    }

                    match std::mem::replace(model, Model::Idle) {
                        Model::Idle => unreachable!(),
                        Model::Active {
                            value,
                            deadline: _,
                            timer,
                        } => UpdateOutcome::new(TaskState::Finished)
                            .with_next_event(value)
                            .with_drop_outside(timer),
                    }
                })
                .unwrap_or(TaskState::Finished)
        });
        let disposal = self.scheduler.run_task(
            task,
            Some(deadline.saturating_duration_since(Instant::now())),
        );

        let mut disposal = Some(disposal);
        self.context.update_flow(move |model| {
            if let Model::Active { timer, .. } = model
                && timer.is_none()
            {
                *timer = disposal.take();
            }
            // If the timer already fired (possible for a zero time span), dispose the returned
            // handle outside the lock.
            UpdateOutcome::empty().with_drop_outside(disposal)
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self
                    .context
                    .update(|model| match std::mem::replace(model, Model::Idle) {
                        Model::Idle => UpdateOutcome::empty()
                            .with_termination_event(completion)
                            .without_drop_outside(),
                        Model::Active {
                            value,
                            deadline: _,
                            timer,
                        } => UpdateOutcome::empty()
                            .with_next_and_termination_events(value, completion)
                            .with_drop_outside(timer),
                    });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
