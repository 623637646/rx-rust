use crate::disposable::{bound_drop_disposal::BoundDropDisposal, chain_disposal::ChainDisposal};
use crate::observable::Subscription;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::{RecursionAction, Scheduler},
};
use educe::Educe;
use std::time::Instant;
use std::{num::NonZeroUsize, time::Duration};

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, either when the bundle reaches a specified size or after a specified time interval, whichever happens first.
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
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = BufferWithTimeOrCount::new(
///         FromIter::new(vec![1, 2, 3]),
///         NonZeroUsize::new(2).unwrap(),
///         Duration::from_millis(10),
///         handle.clone(),
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

impl<T, E, OE, S> Observable<'static, Vec<T>, E> for BufferWithTimeOrCount<OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'static, T, E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = subscribe_with_context::Disposal<'static, ChainDisposal<S::D, OE::D>>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
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
    }
}

struct Model<T> {
    values: Vec<T>,
    last_sending_time_from_counting: Option<Instant>,
}

struct BufferWithTimeOrCountObserver<T, E, OR> {
    context: SubscriptionContext<Vec<T>, E, OR, Model<T>>,
    count: NonZeroUsize,
}

impl<T, E, OR> Observer<T, E> for BufferWithTimeOrCountObserver<T, E, OR>
where
    T: MaybeSend + 'static,
    OR: Observer<Vec<T>, E> + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        let _ = self.context.update_model_and_send(|model| {
            model.values.push(value);
            if model.values.len() >= self.count.get() {
                model.last_sending_time_from_counting = Some(Instant::now());
                let values =
                    std::mem::replace(&mut model.values, Vec::with_capacity(self.count.get()));
                UpdateOutcome::empty().with_next_event(values)
            } else {
                UpdateOutcome::empty().without_events()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update_model_and_send(|model| {
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

/// Drives the periodic flush with a single, long-lived recursive scheduling loop.
///
/// A count-triggered flush (see `BufferWithTimeOrCountObserver::on_next`) doesn't spawn or
/// tear down a task: it just records `last_sending_time_from_counting`, and the next tick of
/// this same loop resyncs its own deadline to `time_span` after that flush instead of emitting.
/// This avoids spawning a fresh scheduler task (and aborting the previous one) on every count
/// flush, and it sidesteps `Duration` subtraction entirely, so a tick that fires late can never
/// panic on underflow.
fn setup_emit_timer<T, E, OR, S>(
    context: SubscriptionContext<Vec<T>, E, OR, Model<T>>,
    scheduler: S,
    delay: Option<Duration>,
    time_span: Duration,
    count: NonZeroUsize,
) -> BoundDropDisposal<S::D>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<Vec<T>, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    assert!(!time_span.is_zero(), "time_span must be non-zero");
    let weak_context = context.downgrade();
    let mut next_time = Instant::now() + delay.unwrap_or_default();
    scheduler.schedule_recursively(
        move |_| {
            let Some(context) = weak_context.upgrade() else {
                return RecursionAction::Stop;
            };
            context
                .update_model_and_send(|model| {
                    if let Some(last_sending_time_from_counting) =
                        model.last_sending_time_from_counting.take()
                    {
                        // Already flushed by count since the last tick; resync to
                        // `time_span` after that flush instead of emitting an empty batch.
                        next_time = last_sending_time_from_counting + time_span;
                        UpdateOutcome::new(RecursionAction::ContinueAt(next_time)).without_events()
                    } else {
                        let values =
                            std::mem::replace(&mut model.values, Vec::with_capacity(count.get()));
                        // Fixed-rate: anchor the next tick to the schedule, not to `now`.
                        next_time += time_span;
                        UpdateOutcome::new(RecursionAction::ContinueAt(next_time))
                            .with_next_event(values)
                    }
                })
                .unwrap_or(RecursionAction::Stop)
        },
        delay,
    )
}
