//! The [`Debounce`] operator, behind
//! [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce).

use crate::delegate_disposal;
use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::thread_mode::{Joined, ThreadMode};
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
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
/// A `time_span` too long for an [`Instant`] to represent never passes: an item is emitted only
/// by the completion, which emits the last one.
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
type DebounceContext<M, T, E, OR, S, D> =
    SubscriptionContext<M, T, E, OR, Model<T, <S as SchedulerTypes>::Disposal>, D>;

/// The task of a [`Debounce`] timer. It holds the context strongly, so that a pending value is
/// still emitted after the source dropped its observer. A disposal still releases the observer at
/// once: the source drops its own handle as it is disposed, and a handle dropped once the context
/// has stopped releases the observer (see `SerializedDelivery`).
type DebounceTask<M, T, E, OR, S, D> = RecursiveContext<DebounceContext<M, T, E, OR, S, D>>;

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
    type Disposal = Disposal<DebounceMode<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Debounce<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<
            DebounceObserver<DebounceMode<OE, S>, T, E, OR, S, <OE as ObservableTypes>::Disposal>,
            Item = T,
            Error = E,
        >,
    S: Scheduler<DebounceTask<DebounceMode<OE, S>, T, E, OR, S, <OE as ObservableTypes>::Disposal>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let model = Model::<T, S::Disposal>::Idle;
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
        /// When `value` is emitted, or `None` when that is too far out for an [`Instant`] to
        /// represent: it then waits for the completion. The clock only moves forward, so a later
        /// value's deadline is `None` too.
        deadline: Option<Instant>,
        timer: Option<BoundDropDisposal<D>>,
    },
}

pub struct DebounceObserver<M, T, E, OR, S, D>
where
    D: Disposable,
    M: ThreadMode,
    S: SchedulerTypes,
{
    context: DebounceContext<M, T, E, OR, S, D>,
    time_span: Duration,
    scheduler: S,
}

impl<M, T, E, OR, S, D> Observer<T, E> for DebounceObserver<M, T, E, OR, S, D>
where
    D: Disposable,
    OR: Observer<T, E>,
    M: ThreadMode,
    S: Scheduler<DebounceTask<M, T, E, OR, S, D>>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let deadline = self.scheduler.now().checked_add(self.time_span);
        let timer_setup = self.context.update(|model| {
            let (timer_setup, previous_value) = match model {
                Model::Idle => {
                    *model = Model::Active {
                        value,
                        deadline,
                        timer: None,
                    };
                    (deadline, None)
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
            // The value replaced the pending one, whose timer is still running, or it never comes
            // due and needs no timer.
            Ok(None) => return Flow::Continue,
            Err(DeliveryStopped) => return Flow::Stop,
        };

        // The model holds only the task's disposal, never the task, which its runtime owns: holding
        // the context strongly forms no cycle. Stopping the context drops the model, which cancels
        // the task.
        let task = Task::recursive(self.context.clone(), |context, _, now| {
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
                    let Some(deadline) = deadline else {
                        // A value with no deadline replaced the one this timer was for: it waits
                        // for the completion, and the timer has nothing left to do.
                        return UpdateOutcome::new(TaskState::Finished)
                            .without_events()
                            .without_drop_outside();
                    };
                    if now < deadline {
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
            Some(deadline.saturating_duration_since(self.scheduler.now())),
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
