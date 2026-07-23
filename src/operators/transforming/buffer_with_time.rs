use crate::disposable::{bound_drop_disposal::BoundDropDisposal, chain_disposal::ChainDisposal};
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

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, after a specified time interval.
/// See <https://reactivex.io/documentation/operators/buffer.html>
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
///             creating::from_iter::FromIter,
///             transforming::buffer_with_time::BufferWithTime,
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
///     let subscription = BufferWithTime::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
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
pub struct BufferWithTime<'or, OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> BufferWithTime<'or, OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            delay,
            _marker: Default::default(),
        }
    }
}

impl<'or, T, E, OE, S> Observable<'static, Vec<T>, E> for BufferWithTime<'or, OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'or, T, E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = subscribe_with_context::Disposal<'or, ChainDisposal<S::D, OE::D>>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        subscribe_with_context(observer, Vec::new(), |context| {
            let sub = self
                .source
                .subscribe(BufferWithTimeObserver(context.clone()));
            let disposal = setup_emit_timer(context, self.scheduler, self.time_span, self.delay);
            sub.preceded_by_bound(disposal)
        })
    }
}

struct BufferWithTimeObserver<T, E, OR>(SubscriptionContext<Vec<T>, E, OR, Vec<T>>);

impl<T, E, OR> Observer<T, E> for BufferWithTimeObserver<T, E, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.try_update_model(|values| {
            values.push(value);
            ModelUpdate::new_empty()
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.try_update_model(|values| {
                    if !values.is_empty() {
                        ModelUpdate::new_without_result()
                            .send_next_and_termination(std::mem::take(values), termination)
                            .ignore_drop_outside()
                    } else {
                        ModelUpdate::new_without_result().send_termination(termination)
                    }
                });
            }
            Termination::Error(_) => self.0.send_termination(termination),
        }
    }
}

fn setup_emit_timer<T, E, OR, S>(
    context: SubscriptionContext<Vec<T>, E, OR, Vec<T>>,
    scheduler: S,
    time_span: Duration,
    delay: Option<Duration>,
) -> BoundDropDisposal<S::D>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<Vec<T>, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    let weak_context = context.downgrade();
    scheduler.schedule_periodically(
        move |_| {
            let Some(context) = weak_context.upgrade() else {
                return false;
            };
            context
                .try_update_model(|values| {
                    ModelUpdate::new(true)
                        .send_next(std::mem::replace(values, Vec::with_capacity(values.len())))
                        .ignore_drop_outside()
                })
                .unwrap_or(false)
        },
        time_span,
        delay,
    )
}
