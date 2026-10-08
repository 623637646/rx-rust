//! The [`BufferWithTime`] operator.

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{PeriodicContext, Scheduler, SchedulerTypes, Task},
    thread_mode::{Joined, ThreadMode},
};
use educe::Educe;
use std::time::Duration;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`,
/// every `time_span`.
///
/// The first bundle is emitted after `delay` — at once, and so empty, for `None` — and every
/// following one `time_span` later, at a fixed rate, empty or not. On completion the pending
/// bundle is emitted before the completion.
///
/// A time too far out for an `Instant` to represent never comes: with a `delay` that long, no
/// bundle is emitted by time, with a `time_span` that long only the first one is. What is gathered
/// after that is emitted by the completion.
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
///             transforming::buffer_with_time::BufferWithTime,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::{Duration, Instant};
///     use tokio::time::sleep;
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = BufferWithTime::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
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
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[vec![1, 2, 3]]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTime<OE, S> {
    /// Creates a [`BufferWithTime`] over `source`;
    /// [`ObservableExt::buffer_with_time`](crate::observable::ObservableExt::buffer_with_time) is
    /// the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            delay,
        }
    }
}

/// The thread mode of a [`BufferWithTime`]: the timer's thread emits the buffers, the source's
/// the last one.
type BufferWithTimeMode<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

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
        BufferWithTimeMode<OE, S>,
        Vec<T>,
        E,
        OR,
        Vec<T>,
        BufferWithTimeSources<OE, S>,
    >,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, Vec<T>, E, Vec<T>, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for BufferWithTime<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = BufferWithTimeMode<OE, S>;
    type Disposal = Disposal<BufferWithTimeMode<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for BufferWithTime<OE, S>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferWithTimeObserver<
                BufferWithTimeMode<OE, S>,
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
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        subscribe_with_context(observer, Vec::new(), |context| {
            let sub = self
                .source
                .subscribe(BufferWithTimeObserver(context.clone()));
            let disposal = setup_emit_timer(context, self.scheduler, self.time_span, self.delay);
            sub.preceded_by_bound(disposal)
        })
        .map_into()
    }
}

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
    context: SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
    scheduler: S,
    time_span: Duration,
    delay: Option<Duration>,
) -> BoundDropDisposal<S::Disposal>
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
        context,
        |context, _| {
            context
                .update(|values| {
                    UpdateOutcome::new(true).with_next_event(std::mem::replace(
                        values,
                        Vec::with_capacity(values.len()),
                    ))
                })
                .unwrap_or(false)
        },
        time_span,
        // Fixed-rate, anchored to the time of the subscription plus `delay`. A delay too long for
        // an `Instant` never ends: the first step never comes, so it needs no anchor.
        scheduler.now().checked_add(delay.unwrap_or_default()),
    );
    scheduler.run_task(task, delay)
}
