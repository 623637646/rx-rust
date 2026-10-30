//! The [`Timeout`] operator, behind
//! [`ObservableExt::timeout`](crate::observable::ObservableExt::timeout).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        option_disposal::OptionDisposal,
    },
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::{DeliveryStopped, UpdateOutcome},
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
    },
};
use educe::Educe;
use std::time::{Duration, Instant};

/// The error type of [`Timeout`]: either the deadline passed, or the source itself failed.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error<E> {
    /// No item arrived within the duration.
    Timeout,
    /// The source terminated with this error.
    SourceError(E),
}

/// Mirrors the source observable, but issues an error if a specified duration elapses between
/// emissions.
/// See <https://reactivex.io/documentation/operators/timeout.html>
///
/// The first item must arrive within `duration` of the subscription, and every later one within
/// `duration` of the item before it; the source's own error is wrapped in
/// [`Error::SourceError`].
///
/// A `duration` too long for an [`Instant`] to represent never elapses: the timeout never fires.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::never::Never, utility::timeout::{Error, Timeout}},
///     scheduler::virtual_time::VirtualTime,
///     thread_mode::mutable::MutableExt,
/// };
/// use std::{sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let termination = Arc::new(Mutex::new(None));
/// let termination_observer = termination.clone();
/// let _subscription = Timeout::new(Never, Duration::from_millis(5), time.scheduler())
///     .subscribe_with_callback(
///         |_| -> () { unreachable!("Never emits no values") },
///         move |end| { termination_observer.replace_value(Some(end)); },
///     );
///
/// time.advance_by(Duration::from_millis(4));
/// assert!(termination.clone_value().is_none());
/// time.advance_by(Duration::from_millis(1));
/// assert_eq!(termination.clone_value(), Some(Termination::Error(Error::Timeout)));
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timeout<OE, S> {
    source: OE,
    duration: Duration,
    scheduler: S,
}

impl<OE, S> Timeout<OE, S> {
    /// Creates a [`Timeout`] over `source`;
    /// [`ObservableExt::timeout`](crate::observable::ObservableExt::timeout) is the fluent form.
    pub fn new(source: OE, duration: Duration, scheduler: S) -> Self {
        Self {
            source,
            duration,
            scheduler,
        }
    }
}

/// The source subscription a [`Timeout`] context owns: the timer, then the source.
type TimeoutSources<OE, S> = ChainDisposal<
    OptionDisposal<<S as SchedulerTypes>::Disposal>,
    <OE as ObservableTypes>::Disposal,
>;

/// The task of a [`Timeout`] timer. It holds the context strongly, so that a source that lets go of
/// its observer without a termination still times out. A disposal still releases the observer at
/// once: disposing the source makes it drop its own handle, and a handle dropped once the context
/// has stopped releases the observer.
type TimeoutTask<T, E, OR, OE, S> = SubscriptionContext<
    JoinedSchedulerModeOf<OE, S>,
    T,
    Error<E>,
    OR,
    Model,
    TimeoutSources<OE, S>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        T,
        Error<E>,
        Model,
        ChainDisposal<OptionDisposal<SD>, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Timeout<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = Error<E>;
    /// The source's thread emits the values, the timer's the timeout, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Timeout<OE, S>
where
    OR: Observer<T, Error<E>>,
    OE: Observable<
            TimeoutObserver<
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                TimeoutSources<OE, S>,
                <S as SchedulerTypes>::Clock,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<TimeoutTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            deadline: self.scheduler.clock().now().checked_add(self.duration),
        };
        subscribe_with_context_two_steps(
            observer,
            model,
            |context| {
                Some(self.source.subscribe(TimeoutObserver {
                    context: context.clone(),
                    duration: self.duration,
                    clock: self.scheduler.clock(),
                }))
            },
            |context| setup_timer(context, &self.scheduler),
        )
        .map_inner_into()
    }
}

/// The state of a [`Timeout`] subscription.
struct Model {
    /// When the timeout fires, or `None` when that is too far out for an [`Instant`] to
    /// represent: the timeout then never fires. The clock only moves forward, so a deadline that
    /// is `None` stays so.
    deadline: Option<Instant>,
}

#[doc(hidden)]
pub struct TimeoutObserver<M: ThreadMode, T, E, OR, D: Disposable, C> {
    context: SubscriptionContext<M, T, Error<E>, OR, Model, D>,
    duration: Duration,
    clock: C,
}

impl<M, T, E, OR, D, C> Observer<T, E> for TimeoutObserver<M, T, E, OR, D, C>
where
    M: ThreadMode,
    OR: Observer<T, Error<E>>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        let deadline = self.clock.now().checked_add(self.duration);
        self.context.update_flow(|model| {
            model.deadline = deadline;
            UpdateOutcome::empty().with_next_event(value)
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.send_termination(match termination {
            Termination::Completed => Termination::Completed,
            Termination::Error(error) => Termination::Error(Error::SourceError(error)),
        });
    }
}

/// Drives the timeout with one long-lived scheduler task.
///
/// Source values only move `deadline` forward. If the task wakes at an obsolete deadline, it
/// continues at the latest one; no scheduler task needs to be cancelled or spawned per value.
fn setup_timer<M, T, E, OR, S, D>(
    context: &SubscriptionContext<M, T, Error<E>, OR, Model, D>,
    scheduler: &S,
) -> Option<DisposeOnDrop<OptionDisposal<S::Disposal>>>
where
    M: ThreadMode,
    OR: Observer<T, Error<E>>,
    S: Scheduler<SubscriptionContext<M, T, Error<E>, OR, Model, D>>,
    D: Disposable,
{
    let deadline = match context.update(|model| UpdateOutcome::new(model.deadline).without_events())
    {
        Ok(Some(deadline)) => deadline,
        // The timeout never fires, so there is no timer to install — but the source stays
        // subscribed, so this step is not the one to end the assembly.
        Ok(None) => return Some(DisposeOnDrop::new(OptionDisposal::none())),
        // The source terminated synchronously while it was being subscribed.
        Err(DeliveryStopped) => return None,
    };

    // The context owns only the task's disposal, never the task, which its runtime owns: holding
    // the context strongly forms no cycle. Stopping the context disposes the timer, which cancels
    // the task.
    let task = Task::new(context.clone(), |context, _, _, now| {
        context
            .update(|model| match model.deadline {
                Some(deadline) if now < deadline => {
                    UpdateOutcome::new(Next::SleepUntil(deadline)).without_events()
                }
                Some(_) => UpdateOutcome::new(Next::Finished)
                    .with_termination_event(Termination::Error(Error::Timeout)),
                // A value moved the deadline out of range: the timeout never fires anymore.
                None => UpdateOutcome::new(Next::Finished).without_events(),
            })
            .unwrap_or(Next::Finished)
    });
    Some(
        scheduler
            .run_task(
                task,
                Some(deadline.saturating_duration_since(scheduler.clock().now())),
            )
            .map_inner(OptionDisposal::some),
    )
}
