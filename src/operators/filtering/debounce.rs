use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    scheduler::{RecursionAction, Scheduler},
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits a notification from the source Observable only after a particular time span has passed without another source emission.
/// See <https://reactivex.io/documentation/operators/debounce.html>
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
///             creating::just::Just,
///             filtering::debounce::Debounce,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = Debounce::new(Just::new(7), Duration::from_millis(5), handle.clone())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[7]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debounce<'or, OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> Debounce<'or, OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            _marker: Default::default(),
        }
    }
}

impl<'or, T, E, OE, S> Observable<'static, T, E> for Debounce<'or, OE, S>
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
        let model = Model::<T, S::D>::Idle;
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(DebounceObserver {
                context,
                time_span: self.time_span,
                scheduler: self.scheduler,
            })
        })
    }
}

/// Keeps at most one recursive scheduler task alive for each debounce burst.
///
/// An active model with `timer: None` covers schedulers that can execute a zero-delay task before
/// returning its disposal.
enum Model<T, D: Disposable> {
    Idle,
    Active {
        value: T,
        deadline: Instant,
        timer: Option<BoundDropDisposal<D>>,
    },
}

struct DebounceObserver<T, E, OR, S: Scheduler> {
    context: SubscriptionContext<T, E, OR, Model<T, S::D>>,
    time_span: Duration,
    scheduler: S,
}

impl<T, E, OR, S: Scheduler> Observer<T, E> for DebounceObserver<T, E, OR, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        let timer_setup = self.context.update(|model| {
            let deadline = Instant::now() + self.time_span;
            let (timer_setup, previous_value) = match model {
                Model::Idle => {
                    *model = Model::Active {
                        value,
                        deadline,
                        timer: None,
                    };
                    (Some(deadline), None)
                }
                Model::Active {
                    value: current_value,
                    deadline: current_deadline,
                    ..
                } => {
                    let previous_value = std::mem::replace(current_value, value);
                    *current_deadline = deadline;
                    (None, Some(previous_value))
                }
            };
            UpdateOutcome::new(timer_setup).with_drop_outside(previous_value)
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
                    .update(|model| {
                        let deadline = match model {
                            Model::Idle => {
                                return UpdateOutcome::new(RecursionAction::Stop)
                                    .without_events()
                                    .without_drop_outside();
                            }
                            Model::Active { deadline, .. } => *deadline,
                        };
                        if Instant::now() < deadline {
                            return UpdateOutcome::new(RecursionAction::ContinueAt(deadline))
                                .without_events()
                                .without_drop_outside();
                        }

                        match std::mem::replace(model, Model::Idle) {
                            Model::Idle => unreachable!(),
                            Model::Active {
                                value,
                                deadline: _,
                                timer,
                            } => UpdateOutcome::new(RecursionAction::Stop)
                                .with_next_event(value)
                                .with_drop_outside(timer),
                        }
                    })
                    .unwrap_or(RecursionAction::Stop)
            },
            Some(deadline.saturating_duration_since(Instant::now())),
        );

        let mut disposal = Some(disposal);
        let _ = self.context.update(move |model| {
            if let Model::Active { timer, .. } = model
                && timer.is_none()
            {
                *timer = disposal.take();
            }
            // If the timer already fired (possible for a zero time span), dispose the returned
            // handle outside the lock.
            UpdateOutcome::empty().with_drop_outside(disposal)
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self
                    .context
                    .update(|model| match std::mem::replace(model, Model::Idle) {
                        Model::Idle => UpdateOutcome::empty()
                            .with_termination_event(completion)
                            .without_drop_outside(),
                        Model::Active {
                            value,
                            deadline: _,
                            timer,
                        } => UpdateOutcome::empty()
                            .with_next_and_termination_events(value, completion)
                            .with_drop_outside(timer),
                    });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
