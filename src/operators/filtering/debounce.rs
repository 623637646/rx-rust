//! The [`Debounce`] operator, behind
//! [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce).

use crate::delegate_disposal;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
};
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
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
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let scheduler = TokioScheduler::current();
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

/// The context of a [`Debounce`] subscription.
type DebounceContext<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T>, D>;

/// The state of the [`Debounce`] timer, one wakeable task for the whole subscription, which the
/// source's observer wakes.
///
/// The task holds the context strongly, so that a pending value is still emitted after the source
/// dropped its observer; it finishes once nothing is pending. A disposal still releases the
/// observer at once: the source drops its own handle as it is disposed, and a handle dropped once
/// the context has stopped releases the observer (see `SerializedDelivery`).
type DebounceTask<M, T, E, OR, D> = WakeableContext<DebounceContext<M, T, E, OR, D>, M>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Debounce<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The timer's thread emits the values, the source's the terminations, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Debounce<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<
            DebounceObserver<
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
        DebounceTask<JoinedSchedulerModeOf<OE, S>, T, E, OR, ChainSchedulerDisposalOf<S, OE>>,
    >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_context(observer, Model::<T>::Idle, |context| {
            let (task, waker) = Task::wakeable(context.clone(), step_timer);
            let source_subscription = self.source.subscribe(DebounceObserver {
                context,
                time_span: self.time_span,
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

/// The state of a [`Debounce`] subscription.
enum Model<T> {
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
/// A value that replaces the pending one only moves the deadline: the timer wakes at the old one,
/// and waits on for the new one. `source_closed` is the `closed` of [`Task::wakeable`]: the source
/// let go of its observer, so it sends nothing more.
fn step_timer<M, T, E, OR, D>(
    context: &mut DebounceContext<M, T, E, OR, D>,
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
        .unwrap_or(Next::Finished)
}

/// The observer a [`Debounce`] gives its source. It holds the scheduler's clock rather than the
/// scheduler, which may be `Local` while the source sends from another thread, and wakes the timer
/// rather than scheduling it.
#[doc(hidden)]
pub struct DebounceObserver<M: ThreadMode, T, E, OR, D: Disposable, C> {
    context: DebounceContext<M, T, E, OR, D>,
    time_span: Duration,
    clock: C,
    waker: TaskWaker<M>,
}

impl<M, T, E, OR, D, C> Observer<T, E> for DebounceObserver<M, T, E, OR, D, C>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        let deadline = self.clock.now().checked_add(self.time_span);
        let wake = self.context.update(|model| match model {
            // The timer waits for the source: wake it, unless the value never comes due.
            Model::Idle => {
                *model = Model::Active { value, deadline };
                UpdateOutcome::new(deadline.is_some()).without_drop_outside()
            }
            // The timer already waits for the pending value's deadline, which this one only
            // moves later.
            Model::Active {
                value: current_value,
                deadline: current_deadline,
            } => {
                let previous_value = std::mem::replace(current_value, value);
                *current_deadline = deadline;
                UpdateOutcome::new(false).with_drop_outside(previous_value)
            }
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
