use crate::disposable::{
    Disposable, bound_drop_disposal::BoundDropDisposal, option_disposal::OptionDisposal,
};
use crate::observable::{Observable, Subscription};
use crate::observer::{Observer, Termination};
use crate::scheduler::{RecursionAction, Scheduler};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, SubscriptionContext, subscribe_with_context_bound_subscription,
};
use crate::utils::types::{MarkerType, MaybeSend};
use educe::Educe;
use std::time::{Duration, Instant};

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error<E> {
    Timeout,
    SourceError(E),
}

/// Mirrors the source Observable, but issues an error if a specified duration elapses between emissions.
/// See <https://reactivex.io/documentation/operators/timeout.html>
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
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let mut subject: PublishSubject<'static, i32, Infallible> = PublishSubject::default();
///
///     let subscription = Timeout::new(subject.clone(), Duration::from_millis(5), handle.clone())
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
pub struct Timeout<'or, OE, S> {
    source: OE,
    duration: Duration,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> Timeout<'or, OE, S> {
    pub fn new(source: OE, duration: Duration, scheduler: S) -> Self {
        Self {
            source,
            duration,
            scheduler,
            _marker: Default::default(),
        }
    }
}

impl<'or, T, E, OE, S> Observable<'static, T, Error<E>> for Timeout<'or, OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = BoundSubscriptionDisposal<'static>;

    fn subscribe(
        self,
        observer: impl Observer<T, Error<E>> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let model = Model {
            deadline: Instant::now() + self.duration,
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            let source_subscription = self.source.subscribe(TimeoutObserver {
                context: context.clone(),
                duration: self.duration,
            });
            let timer = setup_timer(context, &self.scheduler);
            source_subscription.preceded_by(timer)
        })
    }
}

struct Model {
    deadline: Instant,
}

struct TimeoutObserver<T, E, OR, D: Disposable> {
    context: SubscriptionContext<T, Error<E>, OR, Model, D>,
    duration: Duration,
}

impl<T, E, OR, D> Observer<T, E> for TimeoutObserver<T, E, OR, D>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, Error<E>> + MaybeSend + 'static,
    D: Disposable + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        let _ = self.context.update_model_and_send(|model| {
            model.deadline = Instant::now() + self.duration;
            UpdateOutcome::empty().with_next_event(value)
        });
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
fn setup_timer<T, E, OR, S, D>(
    context: SubscriptionContext<T, Error<E>, OR, Model, D>,
    scheduler: &S,
) -> OptionDisposal<BoundDropDisposal<S::D>>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, Error<E>> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
    D: Disposable + MaybeSend + 'static,
{
    let deadline =
        context.update_model_and_send(|model| UpdateOutcome::new(model.deadline).without_events());
    let Ok(deadline) = deadline else {
        // The source terminated synchronously while it was being subscribed.
        return OptionDisposal::none();
    };

    let weak_context = context.downgrade();
    let timer = scheduler.schedule_recursively(
        move |_| {
            let Some(context) = weak_context.upgrade() else {
                return RecursionAction::Stop;
            };
            context
                .update_model_and_send(|model| {
                    if Instant::now() < model.deadline {
                        return UpdateOutcome::new(RecursionAction::ContinueAt(model.deadline))
                            .without_events();
                    }
                    UpdateOutcome::new(RecursionAction::Stop)
                        .with_termination_event(Termination::Error(Error::Timeout))
                })
                .unwrap_or(RecursionAction::Stop)
        },
        Some(deadline.saturating_duration_since(Instant::now())),
    );
    OptionDisposal::some(timer)
}
