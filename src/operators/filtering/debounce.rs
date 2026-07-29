use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::utils::increment_id::IncrementId;
use crate::utils::subscribe_with_context::{
    self, ModelUpdate, SubscriptionContext, subscribe_with_context,
};
use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::time::Duration;

/// Emits a notification from the source Observable only after a particular time span has passed without another source emission.
/// See <https://reactivex.io/documentation/operators/debounce.html>
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {
/// #     panic!("Use tokio-scheduler feature to run tests.");
/// # }
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
    S: Scheduler + MaybeSend + 'or,
{
    type D = subscribe_with_context::Disposal<'or, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let model = Model::<T, S::D> {
            current_value: None,
            timer: None,
            timer_id: IncrementId::default(),
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(DebounceObserver {
                context,
                time_span: self.time_span,
                scheduler: self.scheduler,
            })
        })
    }
}

struct Model<T, D: Disposable> {
    current_value: Option<T>,
    timer: Option<BoundDropDisposal<D>>,
    timer_id: IncrementId,
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
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let timer_id = self.context.try_update_model(|model| {
            model.current_value = Some(value);
            let timer = model.timer.take();
            let timer_id = model.timer_id.increment();
            ModelUpdate::new(timer_id).with_drop_outside(timer)
        });
        let Ok(timer_id) = timer_id else { return };

        let weak_context = self.context.downgrade();
        let disposal = self.scheduler.schedule(
            move || {
                let Some(context) = weak_context.upgrade() else {
                    return;
                };
                let _ = context.try_update_model(|model| {
                    if timer_id != model.timer_id {
                        return ModelUpdate::empty().without_events().without_drop_outside();
                    }
                    let timer = model.timer.take();
                    if let Some(value) = model.current_value.take() {
                        ModelUpdate::empty()
                            .with_next_event(value)
                            .with_drop_outside(timer)
                    } else {
                        ModelUpdate::empty()
                            .without_events()
                            .with_drop_outside(timer)
                    }
                });
            },
            Some(self.time_span),
        );

        let _ = self.context.try_update_model(|model| {
            debug_assert_eq!(
                timer_id, model.timer_id,
                "timer id must be the same because on_next &mut self is exclusive"
            );
            let previous_timer = model.timer.replace(disposal);
            debug_assert!(previous_timer.is_none());
            ModelUpdate::empty()
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.try_update_model(|model| {
                    match (model.current_value.take(), model.timer.take()) {
                        (None, None) => ModelUpdate::empty()
                            .with_termination_event(completion)
                            .without_drop_outside(),
                        (None, Some(timer)) => ModelUpdate::empty()
                            .with_termination_event(completion)
                            .with_drop_outside(timer),
                        (Some(value), None) => ModelUpdate::empty()
                            .with_next_and_termination_events(value, completion)
                            .without_drop_outside(),
                        (Some(value), Some(timer)) => ModelUpdate::empty()
                            .with_next_and_termination_events(value, completion)
                            .with_drop_outside(timer),
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
