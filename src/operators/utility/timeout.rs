//! The [`Timeout`] operator, behind
//! [`ObservableExt::timeout`](crate::observable::ObservableExt::timeout).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::disposable::{
    Disposable, dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal,
};
use crate::observable::{Observable, ObservableTypes};
use crate::observer::{Flow, Observer, Termination};
use crate::scheduler::{RecursiveContext, Scheduler, SchedulerTypes, Task, TaskState};
use crate::thread_mode::{Joined, ThreadMode};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use educe::Educe;
use std::time::{Duration, Instant};

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
/// The error type of [`Timeout`]: either the deadline passed, or the source itself failed.
pub enum Error<E> {
    /// No item arrived within the duration.
    Timeout,
    /// The source terminated with this error.
    SourceError(E),
}

/// Mirrors the source Observable, but issues an error if a specified duration elapses between
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
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::ObservableExt,
///         observer::{Observer, Termination},
///         operators::utility::timeout::{Error, Timeout},
///         subject::publish_subject::PublishSubject,
///     };
///     use std::{convert::Infallible, sync::{Arc, Mutex}};
///     use tokio::time::{sleep, Duration};
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let mut subject: PublishSubject<'static, i32, Infallible, rx_rust::thread_mode::Shared> = PublishSubject::shared();
///
///     let subscription = Timeout::new(subject.clone(), Duration::from_millis(5), scheduler.clone())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
///
///     subject.on_next(1);
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[1]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Error(Error::Timeout)]
///     );
/// }
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

/// The thread mode of a [`Timeout`]: the source's thread emits the values, the timer's the
/// timeout.
type TimeoutMode<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The source subscription a [`Timeout`] context owns: the timer, then the source.
type TimeoutSources<OE, S> = ChainDisposal<
    OptionDisposal<DisposeOnDrop<<S as SchedulerTypes>::Disposal>>,
    <OE as ObservableTypes>::Disposal,
>;

/// The task of a [`Timeout`] timer. It holds the context strongly, so that a source that lets go of
/// its observer without a termination still times out. A disposal still releases the observer at
/// once: disposing the source makes it drop its own handle, and a handle dropped once the context
/// has stopped releases the observer.
type TimeoutTask<T, E, OR, OE, S> = RecursiveContext<
    SubscriptionContext<TimeoutMode<OE, S>, T, Error<E>, OR, Model, TimeoutSources<OE, S>>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, T, Error<E>, Model, ChainDisposal<OptionDisposal<DisposeOnDrop<SD>>, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for Timeout<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = Error<E>;
    type Mode = TimeoutMode<OE, S>;
    type Disposal = Disposal<TimeoutMode<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Timeout<OE, S>
where
    OR: Observer<T, Error<E>>,
    OE: Observable<
            TimeoutObserver<TimeoutMode<OE, S>, T, E, OR, TimeoutSources<OE, S>, S>,
            Item = T,
            Error = E,
        >,
    S: Scheduler<TimeoutTask<T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            deadline: self.scheduler.now().checked_add(self.duration),
        };
        subscribe_with_context(observer, model, |context| {
            let source_subscription = self.source.subscribe(TimeoutObserver {
                context: context.clone(),
                duration: self.duration,
                scheduler: self.scheduler.clone(),
            });
            let timer = setup_timer(context, &self.scheduler);
            source_subscription.preceded_by(timer)
        })
        .map_into()
    }
}

/// The state of a [`Timeout`] subscription.
struct Model {
    /// When the timeout fires, or `None` when that is too far out for an [`Instant`] to
    /// represent: the timeout then never fires. The clock only moves forward, so a deadline that
    /// is `None` stays so.
    deadline: Option<Instant>,
}

pub struct TimeoutObserver<M: ThreadMode, T, E, OR, D: Disposable, S> {
    context: SubscriptionContext<M, T, Error<E>, OR, Model, D>,
    duration: Duration,
    scheduler: S,
}

impl<M, T, E, OR, D, S> Observer<T, E> for TimeoutObserver<M, T, E, OR, D, S>
where
    M: ThreadMode,
    OR: Observer<T, Error<E>>,
    D: Disposable,
    S: SchedulerTypes,
{
    fn on_next(&mut self, value: T) -> Flow {
        let deadline = self.scheduler.now().checked_add(self.duration);
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

/// Drives the timeout with one long-lived recursive scheduler task.
///
/// Source values only move `deadline` forward. If the task wakes at an obsolete deadline, it
/// continues at the latest one; no scheduler task needs to be cancelled or spawned per value.
fn setup_timer<M, T, E, OR, S, D>(
    context: SubscriptionContext<M, T, Error<E>, OR, Model, D>,
    scheduler: &S,
) -> OptionDisposal<DisposeOnDrop<S::Disposal>>
where
    M: ThreadMode,
    OR: Observer<T, Error<E>>,
    S: Scheduler<RecursiveContext<SubscriptionContext<M, T, Error<E>, OR, Model, D>>>,
    D: Disposable,
{
    let deadline = context.update(|model| UpdateOutcome::new(model.deadline).without_events());
    // `Err`: the source terminated synchronously while it was being subscribed. `Ok(None)`: the
    // timeout never fires, so no timer is needed.
    let Ok(Some(deadline)) = deadline else {
        return OptionDisposal::none();
    };

    // The context owns only the task's disposal, never the task, which its runtime owns: holding
    // the context strongly forms no cycle. Stopping the context disposes the timer, which cancels
    // the task.
    let task = Task::recursive(context, |context, _, now| {
        context
            .update(|model| match model.deadline {
                Some(deadline) if now < deadline => {
                    UpdateOutcome::new(TaskState::SleepUntil(deadline)).without_events()
                }
                Some(_) => UpdateOutcome::new(TaskState::Finished)
                    .with_termination_event(Termination::Error(Error::Timeout)),
                // A value moved the deadline out of range: the timeout never fires anymore.
                None => UpdateOutcome::new(TaskState::Finished).without_events(),
            })
            .unwrap_or(TaskState::Finished)
    });
    let timer = scheduler.run_task(
        task,
        Some(deadline.saturating_duration_since(scheduler.now())),
    );
    OptionDisposal::some(timer)
}
