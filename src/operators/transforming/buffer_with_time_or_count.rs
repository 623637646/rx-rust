//! The [`BufferWithTimeOrCount`] operator.

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::disposable::{
    Disposable, dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal,
};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
};
use educe::Educe;
use std::time::Instant;
use std::{num::NonZeroUsize, time::Duration};

/// Gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, either when
/// the bundle reaches `count` items or every `time_span`, whichever happens first.
///
/// The timer first fires one `time_span` after the subscription, as ReactiveX does. A bundle
/// emitted because it is full restarts the timer, so the next timed bundle comes `time_span` after
/// it.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long,
/// bundles are emitted by count and by the completion only.
///
/// # Panics
///
/// Subscribing panics if `time_span` is zero.
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
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    /// Creates a [`BufferWithTimeOrCount`] over `source`.
    pub fn new(source: OE, count: NonZeroUsize, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
        }
    }
}

/// The context of a [`BufferWithTimeOrCount`] subscription.
type BufferWithTimeOrCountContext<M, T, E, OR, D> =
    SubscriptionContext<M, Vec<T>, E, OR, Model<T>, D>;

/// The source subscription a [`BufferWithTimeOrCount`] context owns: the timer, then the source.
type BufferWithTimeOrCountSources<OE, S> = ChainDisposal<
    OptionDisposal<DisposeOnDrop<<S as SchedulerTypes>::Disposal>>,
    <OE as ObservableTypes>::Disposal,
>;

/// The task of a [`BufferWithTimeOrCount`] timer. It holds the context strongly, so that the
/// buffers keep being cut after the source has let go of its observer without a termination, until
/// the subscription is disposed. A disposal still releases the observer at once: disposing the
/// source makes it drop its own handle, and a handle dropped once the context has stopped releases
/// the observer.
type BufferWithTimeOrCountTask<T, E, OR, OE, S> = BufferWithTimeOrCountContext<
    JoinedSchedulerModeOf<OE, S>,
    T,
    E,
    OR,
    BufferWithTimeOrCountSources<OE, S>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, Vec<T>, E, Model<T>, ChainDisposal<OptionDisposal<DisposeOnDrop<SD>>, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTimeOrCount<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    /// The timer's thread and the source's both emit buffers, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTimeOrCount<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeOrCountObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                BufferWithTimeOrCountSources<OE, S>,
                <S as SchedulerTypes>::Clock,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeOrCountTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        assert!(!self.time_span.is_zero(), "time_span must be non-zero");
        let model = Model::<T> {
            values: Vec::with_capacity(self.count.get()),
            deadline: self.scheduler.clock().now().checked_add(self.time_span),
            time_span: self.time_span,
            count: self.count,
        };
        subscribe_with_context(observer, model, |context| {
            let buffer_observer = BufferWithTimeOrCountObserver {
                context: context.clone(),
                clock: self.scheduler.clock(),
            };
            let sub = self.source.subscribe(buffer_observer);
            let timer = setup_emit_timer(context, self.scheduler, self.time_span);
            sub.preceded_by(timer)
        })
        .map_inner_into()
    }
}

/// The state of a [`BufferWithTimeOrCount`] subscription.
struct Model<T> {
    values: Vec<T>,
    /// When the next timed bundle is due: `time_span` after the subscription, then after the
    /// bundle before, timed or full. `None` when that is too far out for an [`Instant`] to
    /// represent: it never comes, nor does any later one, since the clock only moves forward, and
    /// the bundles are emitted by count and by the completion only.
    deadline: Option<Instant>,
    /// Fixed; here for the timer task, whose handler is a plain `fn`.
    time_span: Duration,
    count: NonZeroUsize,
}

impl<T> Model<T> {
    /// Takes the bundle out, leaving an empty one of the same capacity.
    fn take_bundle(&mut self) -> Vec<T> {
        std::mem::replace(&mut self.values, Vec::with_capacity(self.count.get()))
    }
}

/// The observer a [`BufferWithTimeOrCount`] gives its source. It holds the scheduler's clock rather
/// than the scheduler, which may be `Local` while the source sends from another thread.
pub struct BufferWithTimeOrCountObserver<M: ThreadMode, T, E, OR, D: Disposable, C> {
    context: BufferWithTimeOrCountContext<M, T, E, OR, D>,
    clock: C,
}

impl<M, T, E, OR, D, C> Observer<T, E> for BufferWithTimeOrCountObserver<M, T, E, OR, D, C>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.context.update_flow(|model| {
            model.values.push(value);
            if model.values.len() < model.count.get() {
                return UpdateOutcome::empty().without_events();
            }
            // A full bundle restarts the timer: the next timed bundle comes `time_span` after it,
            // which is never before the one already due, itself at most `time_span` after the
            // last tick. Out of range means never, as it already did for the deadline due.
            model.deadline = self.clock.now().checked_add(model.time_span);
            UpdateOutcome::empty().with_next_event(model.take_bundle())
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

/// Drives the timed bundles with one long-lived scheduler task, as `timeout` does.
///
/// A full bundle only moves `deadline` (see `BufferWithTimeOrCountObserver::on_next`): if the task
/// wakes at an obsolete deadline, it sleeps on to the current one. No scheduler task is spawned or
/// cancelled per full bundle, and no `Duration` is subtracted, so a tick that fires late can never
/// panic on underflow.
fn setup_emit_timer<M, T, E, OR, D, S>(
    context: BufferWithTimeOrCountContext<M, T, E, OR, D>,
    scheduler: S,
    time_span: Duration,
) -> OptionDisposal<DisposeOnDrop<S::Disposal>>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
    S: Scheduler<BufferWithTimeOrCountContext<M, T, E, OR, D>>,
{
    // The source terminated synchronously while it was being subscribed: no bundle is left to cut.
    if context
        .update(|_| UpdateOutcome::new(()).without_events())
        .is_err()
    {
        return OptionDisposal::none();
    }

    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::new(context, |context, _, _, now| {
        context
            .update(|model| match model.deadline {
                Some(deadline) if now < deadline => {
                    UpdateOutcome::new(Next::SleepUntil(deadline)).without_events()
                }
                Some(deadline) => {
                    // Fixed-rate: the next deadline is counted from this one, not from `now`, so
                    // that a late tick does not push the next ones back.
                    model.deadline = deadline.checked_add(model.time_span);
                    let state = model.deadline.map_or(Next::Finished, Next::SleepUntil);
                    UpdateOutcome::new(state).with_next_event(model.take_bundle())
                }
                // A full bundle moved the deadline out of range: no timed bundle is left.
                None => UpdateOutcome::new(Next::Finished).without_events(),
            })
            .unwrap_or(Next::Finished)
    });
    OptionDisposal::some(scheduler.run_task(task, Some(time_span)))
}
