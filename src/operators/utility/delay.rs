use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::utils::subscription_slot::SubscriptionSlot;
use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    scheduler::{RecursionAction, Scheduler},
};
use educe::Educe;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Shifts the emissions from an Observable forward in time by a specified duration.
/// See <https://reactivex.io/documentation/operators/delay.html>
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
///             utility::delay::Delay,
///         },
///     };
///     use std::{
///         sync::{Arc, Mutex},
///         time::Duration,
///     };
///     use tokio::time::sleep;
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = Delay::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
///         handle.clone(),
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
///     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<'or, OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> Delay<'or, OE, S> {
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
            _marker: Default::default(),
        }
    }
}

impl<'or, T, E, OE, S> Observable<'static, T, E> for Delay<'or, OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'or, T, E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = subscribe_with_context::Disposal<'or, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let model = Model::<T, S::D> {
            values: VecDeque::new(),
            completion: None,
            timer: SubscriptionSlot::Idle,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(DelayObserver {
                context,
                delay: self.delay,
                scheduler: self.scheduler,
            })
        })
    }
}

/// The events waiting for their deadline, and the recursive scheduler task that delivers them.
struct Model<T, D: Disposable> {
    /// The values with the instant at which each of them is due, in ascending order.
    values: VecDeque<(Instant, T)>,
    /// The instant at which the completion is due, once the source has completed.
    completion: Option<Instant>,
    /// Keeps at most one recursive scheduler task alive while events are waiting. The slot is
    /// reserved while the task is being scheduled, which covers schedulers that can execute a
    /// zero-delay task before returning its disposal.
    timer: SubscriptionSlot<BoundDropDisposal<D>>,
}

impl<T, D: Disposable> Model<T, D> {
    /// Returns the instant at which the next event is due.
    fn next_deadline(&self) -> Option<Instant> {
        self.values
            .front()
            .map(|(deadline, _)| *deadline)
            .or(self.completion)
    }
}

struct DelayObserver<T, E, OR, S: Scheduler> {
    context: SubscriptionContext<T, E, OR, Model<T, S::D>>,
    delay: Duration,
    scheduler: S,
}

impl<T, E, OR, S> DelayObserver<T, E, OR, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    /// Queues `value`, or the completion when it is `None`, and starts the timer if needed.
    fn queue_event(&self, value: Option<T>) {
        let timer_setup = self.context.update_model_and_send(|model| {
            let deadline = Instant::now() + self.delay;
            match value {
                Some(value) => model.values.push_back((deadline, value)),
                None => model.completion = Some(deadline),
            }
            let start_timer = model.timer.reserve_if_idle();
            UpdateOutcome::new(start_timer.then_some(deadline))
        });
        let Ok(Some(deadline)) = timer_setup else {
            return;
        };

        let weak_context = self.context.downgrade();
        let disposal = self.scheduler.schedule_recursively(
            move |_| {
                let Some(context) = weak_context.upgrade() else {
                    return RecursionAction::Stop;
                };
                context
                    .update_model_and_send(|model| {
                        let now = Instant::now();
                        // The deadlines are ascending, so one binary search splits the queue into
                        // the due values and the ones that keep waiting.
                        let due = model
                            .values
                            .partition_point(|(deadline, _)| *deadline <= now);
                        let values: Vec<T> =
                            model.values.drain(..due).map(|(_, value)| value).collect();

                        // The completion is queued last, so it is due only once no value is left
                        // to deliver before it and its own deadline has passed. A value that
                        // still waits holds the completion back, which keeps the order right.
                        if model.values.is_empty()
                            && model.completion.is_some_and(|deadline| deadline <= now)
                        {
                            return UpdateOutcome::new(RecursionAction::Stop)
                                .with_events(EventBatch::NextBatchAndTermination(
                                    values,
                                    Termination::Completed,
                                ))
                                .with_drop_outside(model.timer.release());
                        }

                        match model.next_deadline() {
                            Some(deadline) => {
                                UpdateOutcome::new(RecursionAction::ContinueAt(deadline))
                                    .with_events(EventBatch::NextBatch(values))
                                    .without_drop_outside()
                            }
                            // Nothing is waiting anymore: stop the timer until the next event.
                            None => UpdateOutcome::new(RecursionAction::Stop)
                                .with_events(EventBatch::NextBatch(values))
                                .with_drop_outside(model.timer.release()),
                        }
                    })
                    .unwrap_or(RecursionAction::Stop)
            },
            Some(deadline.saturating_duration_since(Instant::now())),
        );

        let _ = self.context.update_model_and_send(move |model| {
            // If the timer already stopped (possible for a zero delay), `fill` gives the handle
            // back to dispose outside the lock.
            UpdateOutcome::empty().with_drop_outside(model.timer.fill(disposal))
        });
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, E, OR, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        self.queue_event(Some(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => self.queue_event(None),
            // An error is not delayed: it terminates the subscription right away, which drops the
            // values that are still waiting along with the timer.
            error @ Termination::Error(_) => self.context.send_termination(error),
        }
    }
}
