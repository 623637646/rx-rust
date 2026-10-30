//! The [`BufferWithTime`] operator.

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, PeriodicContext, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
        take_leaving_room,
    },
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Periodically gathers items from an observable into bundles and emits these bundles as `Vec<T>`,
/// every `time_span`.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The first bundle is emitted one `time_span` after the subscription, as ReactiveX does, and
/// every following one `time_span` later, at a fixed rate, empty or not. On completion the pending
/// bundle is emitted before the completion.
/// A late timer cuts overdue bundles without waiting another span, keeping the original rate.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long, no
/// bundle is emitted by time, and what is gathered is emitted by the completion.
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
///     operators::transforming::buffer_with_time::BufferWithTime,
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
/// let _subscription = BufferWithTime::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// time.advance_by(span);
/// assert_eq!(values.clone_value(), [vec![1, 2]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> BufferWithTime<OE, S> {
    /// Creates a [`BufferWithTime`] over `source`;
    /// [`ObservableExt::buffer_with_time`](crate::observable::ObservableExt::buffer_with_time) is
    /// the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

/// The source subscription a [`BufferWithTime`] context owns: the timer, then the source.
type BufferWithTimeSources<OE, S> =
    ChainDisposal<<S as SchedulerTypes>::Disposal, <OE as ObservableTypes>::Disposal>;

/// The task of a [`BufferWithTime`] timer. It holds the context strongly, so that the buffers keep
/// being cut after the source has let go of its observer without a termination, until the
/// subscription is disposed. A disposal still releases the observer at once: disposing the source
/// makes it drop its own handle, and a handle dropped once the context has stopped releases the
/// observer.
type BufferWithTimeTask<T, E, OR, OE, S> = PeriodicContext<
    SubscriptionContext<
        JoinedSchedulerModeOf<OE, S>,
        Vec<T>,
        E,
        OR,
        Vec<T>,
        BufferWithTimeSources<OE, S>,
    >,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        Vec<T>,
        E,
        Vec<T>,
        ChainDisposal<SD, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTime<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    /// The timer's thread emits the buffers, the source's the last one, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTime<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                BufferWithTimeSources<OE, S>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<BufferWithTimeTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        assert!(!self.time_span.is_zero(), "time_span must be non-zero");
        let deadline = self.scheduler.clock().now().checked_add(self.time_span);
        subscribe_with_context_two_steps(
            observer,
            Vec::new(),
            |context| {
                Some(
                    self.source
                        .subscribe(BufferWithTimeObserver(context.clone())),
                )
            },
            |context| {
                Some(setup_emit_timer(
                    context,
                    self.scheduler,
                    self.time_span,
                    deadline,
                ))
            },
        )
        .map_inner_into()
    }
}

#[doc(hidden)]
pub struct BufferWithTimeObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
);

impl<M, T, E, OR, D> Observer<T, E> for BufferWithTimeObserver<M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.update_flow(|values| {
            values.push(value);
            UpdateOutcome::empty().without_events()
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|values| {
                    if !values.is_empty() {
                        UpdateOutcome::empty()
                            .with_next_and_termination_events(std::mem::take(values), completion)
                    } else {
                        UpdateOutcome::empty().with_termination_event(completion)
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.0.send_termination(error);
            }
        }
    }
}

fn setup_emit_timer<M, T, E, OR, D, S>(
    context: &SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
    scheduler: S,
    time_span: Duration,
    deadline: Option<Instant>,
) -> DisposeOnDrop<S::Disposal>
where
    M: ThreadMode,
    OR: Observer<Vec<T>, E>,
    D: Disposable,
    S: Scheduler<PeriodicContext<SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>>>,
{
    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::periodic(
        context.clone(),
        |context, _| {
            context
                .update(|values| {
                    UpdateOutcome::new(true)
                        .with_next_event(take_leaving_room(values, values.len()))
                })
                .unwrap_or(false)
        },
        time_span,
        deadline,
    );
    // The task waits for its absolute anchor, even if subscribing or its first poll ran late.
    // An unrepresentable deadline keeps the task parked until disposal.
    let delay = deadline.is_none().then_some(time_span);
    scheduler.run_task(task, delay)
}
