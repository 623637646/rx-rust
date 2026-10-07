//! The [`BufferWithTimeOrCount`] operator.

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::observable::Subscription;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{RecursiveContext, Scheduler, SchedulerTypes, Task, TaskState},
    thread_mode::{Joined, ThreadMode},
};
use educe::Educe;
use std::time::Instant;
use std::{num::NonZeroUsize, time::Duration};

/// Gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, either when
/// the bundle reaches `count` items or every `time_span`, whichever happens first.
///
/// The timer first fires after `delay` (at once for `None`). A bundle emitted because it is full
/// restarts the timer, so the next timed bundle comes `time_span` after it.
/// See <https://reactivex.io/documentation/operators/buffer.html>
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
///             transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
///         },
///     };
///     use std::{
///         num::NonZeroUsize,
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
///     let subscription = BufferWithTimeOrCount::new(
///         FromIter::new(vec![1, 2, 3]),
///         NonZeroUsize::new(2).unwrap(),
///         Duration::from_millis(10),
///         scheduler.clone(),
///         None,
///     )
///     .subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
///     sleep(Duration::from_millis(20)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[vec![1, 2], vec![3]]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeOrCount<OE, S> {
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    /// Creates a [`BufferWithTimeOrCount`] over `source`.
    pub fn new(
        source: OE,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
            delay,
        }
    }
}

/// The thread mode of a [`BufferWithTimeOrCount`]: the timer's thread and the source's both emit
/// buffers.
type BufferWithTimeOrCountMode<OE, S> =
    Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The source subscription a [`BufferWithTimeOrCount`] context owns: the timer, then the source.
type BufferWithTimeOrCountSources<OE, S> =
    ChainDisposal<<S as SchedulerTypes>::Disposal, <OE as ObservableTypes>::Disposal>;

/// The task of a [`BufferWithTimeOrCount`] timer.
type BufferWithTimeOrCountTask<T, E, OR, OE, S> = RecursiveContext<
    EmitTimer<
        SubscriptionContext<
            BufferWithTimeOrCountMode<OE, S>,
            Vec<T>,
            E,
            OR,
            Model<T>,
            BufferWithTimeOrCountSources<OE, S>,
        >,
    >,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, Vec<T>, E, Model<T>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTimeOrCount<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = BufferWithTimeOrCountMode<OE, S>;
    type Disposal = Disposal<BufferWithTimeOrCountMode<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTimeOrCount<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeOrCountObserver<
                BufferWithTimeOrCountMode<OE, S>,
                T,
                E,
                OR,
                BufferWithTimeOrCountSources<OE, S>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeOrCountTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let model = Model::<T> {
            values: Vec::with_capacity(self.count.get()),
            last_sending_time_from_counting: None,
        };
        subscribe_with_context(observer, model, |context| {
            let buffer_observer = BufferWithTimeOrCountObserver {
                context: context.clone(),
                count: self.count,
            };
            let sub = self.source.subscribe(buffer_observer);
            let disposal = setup_emit_timer(
                context,
                self.scheduler,
                self.delay,
                self.time_span,
                self.count,
            );
            sub.preceded_by_bound(disposal)
        })
        .map_into()
    }
}

struct Model<T> {
    values: Vec<T>,
    last_sending_time_from_counting: Option<Instant>,
}

pub struct BufferWithTimeOrCountObserver<M: ThreadMode, T, E, OR, D: Disposable> {
    context: SubscriptionContext<M, Vec<T>, E, OR, Model<T>, D>,
    count: NonZeroUsize,
}

impl<M, T, E, OR, D> Observer<T, E> for BufferWithTimeOrCountObserver<M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.context.update_flow(|model| {
            model.values.push(value);
            if model.values.len() >= self.count.get() {
                model.last_sending_time_from_counting = Some(Instant::now());
                let values =
                    std::mem::replace(&mut model.values, Vec::with_capacity(self.count.get()));
                UpdateOutcome::empty().with_next_event(values)
            } else {
                UpdateOutcome::empty().without_events()
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update(|model| {
                    if !model.values.is_empty() {
                        let values = std::mem::take(&mut model.values);
                        UpdateOutcome::empty().with_next_and_termination_events(values, completion)
                    } else {
                        UpdateOutcome::empty().with_termination_event(completion)
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}

/// The state of a [`BufferWithTimeOrCount`] timer: the context and the schedule.
///
/// The context is held strongly, so that the buffers keep being cut after the source has let go of
/// its observer without a termination, until the subscription is disposed. A disposal still
/// releases the observer at once: disposing the source makes it drop its own handle, and a handle
/// dropped once the context has stopped releases the observer.
struct EmitTimer<C> {
    context: C,
    next_time: Instant,
    time_span: Duration,
    count: NonZeroUsize,
}

/// Drives the periodic flush with a single, long-lived recursive scheduling loop.
///
/// A count-triggered flush (see `BufferWithTimeOrCountObserver::on_next`) doesn't spawn or
/// tear down a task: it just records `last_sending_time_from_counting`, and the next tick of
/// this same loop resyncs its own deadline to `time_span` after that flush instead of emitting.
/// This avoids spawning a fresh scheduler task (and aborting the previous one) on every count
/// flush, and it sidesteps `Duration` subtraction entirely, so a tick that fires late can never
/// panic on underflow.
fn setup_emit_timer<M, T, E, OR, D, S>(
    context: SubscriptionContext<M, Vec<T>, E, OR, Model<T>, D>,
    scheduler: S,
    delay: Option<Duration>,
    time_span: Duration,
    count: NonZeroUsize,
) -> BoundDropDisposal<S::Disposal>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
    S: Scheduler<RecursiveContext<EmitTimer<SubscriptionContext<M, Vec<T>, E, OR, Model<T>, D>>>>,
{
    assert!(!time_span.is_zero(), "time_span must be non-zero");
    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let timer = EmitTimer {
        context,
        next_time: Instant::now() + delay.unwrap_or_default(),
        time_span,
        count,
    };
    let task = Task::recursive(timer, |timer, _| {
        let (time_span, count) = (timer.time_span, timer.count);
        let next_time = &mut timer.next_time;
        timer
            .context
            .update(|model| {
                if let Some(last_sending_time_from_counting) =
                    model.last_sending_time_from_counting.take()
                {
                    // Already flushed by count since the last tick; resync to
                    // `time_span` after that flush instead of emitting an empty batch.
                    *next_time = last_sending_time_from_counting + time_span;
                    UpdateOutcome::new(TaskState::SleepUntil(*next_time)).without_events()
                } else {
                    let values =
                        std::mem::replace(&mut model.values, Vec::with_capacity(count.get()));
                    // Fixed-rate: anchor the next tick to the schedule, not to `now`.
                    *next_time += time_span;
                    UpdateOutcome::new(TaskState::SleepUntil(*next_time)).with_next_event(values)
                }
            })
            .unwrap_or(TaskState::Finished)
    });
    scheduler.run_task(task, delay)
}
