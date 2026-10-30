//! The [`BufferWithTimeOrCount`] operator.

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
        take_leaving_room,
    },
};
use educe::Educe;
use std::time::Instant;
use std::{num::NonZeroUsize, time::Duration};

/// Gathers items from an observable into bundles and emits these bundles as `Vec<T>`, either when
/// the bundle reaches `count` items or every `time_span`, whichever happens first.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The timer first fires one `time_span` after the subscription, as ReactiveX does. A bundle
/// emitted because it is full restarts the timer, so the next timed bundle comes `time_span` after
/// it.
/// A late timer cuts overdue bundles without waiting another span, unless a full bundle reset it.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long,
/// bundles are emitted by count and by the completion only.
///
/// # Panics
///
/// Subscribing panics if `time_span` is zero.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let span = Duration::from_millis(5);
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = values.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription = BufferWithTimeOrCount::new(
///     source.clone(), std::num::NonZeroUsize::new(2).unwrap(), span, time.scheduler(),
/// )
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2); // Full: emitted immediately.
/// let _ = source.on_next(3);
/// assert_eq!(values.clone_value(), [vec![1, 2]]);
/// time.advance_by(span);
/// assert_eq!(values.clone_value(), [vec![1, 2], vec![3]]);
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
    /// Creates a [`BufferWithTimeOrCount`] over `source`;
    /// [`ObservableExt::buffer_with_time_or_count`](crate::observable::ObservableExt::buffer_with_time_or_count)
    /// is the fluent form.
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
type BufferWithTimeOrCountSources<OE, S> =
    ChainDisposal<<S as SchedulerTypes>::Disposal, <OE as ObservableTypes>::Disposal>;

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
    subscribe_with_context::Disposal<
        M,
        Vec<T>,
        E,
        Model<T>,
        ChainDisposal<SD, D>,
    >,
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
        let model = Model {
            values: Vec::with_capacity(self.count.get()),
            deadline: self.scheduler.clock().now().checked_add(self.time_span),
            time_span: self.time_span,
            count: self.count,
        };
        let clock = self.scheduler.clock();
        subscribe_with_context_two_steps(
            observer,
            model,
            |context| {
                Some(self.source.subscribe(BufferWithTimeOrCountObserver {
                    context: context.clone(),
                    clock,
                }))
            },
            |context| setup_emit_timer(context, self.scheduler, self.time_span),
        )
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
    /// Takes the bundle out, leaving room for a full one.
    fn take_bundle(&mut self) -> Vec<T> {
        take_leaving_room(&mut self.values, self.count.get())
    }
}

/// The observer a [`BufferWithTimeOrCount`] gives its source. It holds the scheduler's clock rather
/// than the scheduler, which may be `Local` while the source sends from another thread.
#[doc(hidden)]
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
    context: &BufferWithTimeOrCountContext<M, T, E, OR, D>,
    scheduler: S,
    time_span: Duration,
) -> Option<DisposeOnDrop<S::Disposal>>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
    S: Scheduler<BufferWithTimeOrCountContext<M, T, E, OR, D>>,
{
    // The source terminated synchronously while it was being subscribed: no bundle is left to cut.
    let Ok(deadline) = context.update(|model| UpdateOutcome::new(model.deadline).without_events())
    else {
        return None;
    };

    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::new(context.clone(), |context, _, _, now| {
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
    // Let the handler wait for the absolute deadline, rather than starting a relative delay
    // on its first poll. An unrepresentable deadline keeps the task parked until disposal.
    let delay = deadline.is_none().then_some(time_span);
    Some(scheduler.run_task(task, delay))
}
