use crate::disposable::{Disposable, bound_drop_disposal::BoundDropDisposal};
use crate::observable::Subscription;
use crate::utils::subscribe_with_context::{
    self, Context, ModificationResult, subscribe_with_context,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
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
    type D = subscribe_with_context::Disposal<'static, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let model = Model::<T, S::D> {
            values: Vec::with_capacity(self.count.get()),
            timer: None,
            last_sending_time_from_counting: None,
        };
        subscribe_with_context(observer, model, |context| {
            let buffer_observer = BufferWithTimeOrCountObserver {
                context: context.clone(),
                count: self.count,
            };
            let sub = self.source.subscribe(buffer_observer);
            let disposal = setup_emit_timer(
                context.clone(),
                self.scheduler,
                self.delay,
                self.time_span,
                self.count,
            );
            let _ = context.modify_model(|model| {
                model.timer = Some(disposal);
                ModificationResult::new_empty()
            });
            sub
        })
    }
}

struct Model<T, D: Disposable> {
    values: Vec<T>,
    timer: Option<BoundDropDisposal<D>>,
    last_sending_time_from_counting: Option<Instant>,
}

struct BufferWithTimeOrCountObserver<T, E, OR, D: Disposable> {
    context: Context<Vec<T>, E, OR, Model<T, D>>,
    count: NonZeroUsize,
}

impl<T, E, OR, D: Disposable> Observer<T, E> for BufferWithTimeOrCountObserver<T, E, OR, D>
where
    T: MaybeSend + 'static,
    OR: Observer<Vec<T>, E> + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        let _ = self.context.modify_model(|model| {
            model.values.push(value);
            if model.values.len() >= self.count.get() {
                model.last_sending_time_from_counting = Some(Instant::now());
                let values =
                    std::mem::replace(&mut model.values, Vec::with_capacity(self.count.get()));
                ModificationResult::new_send_next(values)
            } else {
                ModificationResult::new_empty()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.context.modify_model(|model| {
                    if !model.values.is_empty() {
                        let values = std::mem::take(&mut model.values);
                        ModificationResult::new_send_next_and_termination(values, termination)
                    } else {
                        ModificationResult::new_send_termination(termination)
                    }
                });
            }
            Termination::Error(_) => {
                self.context.send_termination(termination);
            }
        }
    }
}

fn setup_emit_timer<T, E, OR, S>(
    context: Context<Vec<T>, E, OR, Model<T, S::D>>,
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
    let weak_context = context.downgrade();
    scheduler.clone().schedule_periodically(
        move |_| {
            let Some(context) = weak_context.upgrade() else {
                return false;
            };
            context
                .modify_model(|model| {
                    if let Some(last_sending_time_from_counting) =
                        model.last_sending_time_from_counting.take()
                    {
                        let disposal = setup_emit_timer(
                            context.clone(),
                            scheduler.clone(),
                            Some(time_span - last_sending_time_from_counting.elapsed()), // TODO: May be panic.
                            time_span,
                            count,
                        );
                        let old_timer = model.timer.replace(disposal);
                        ModificationResult::new(false).drop_outside(old_timer)
                    } else {
                        let values =
                            std::mem::replace(&mut model.values, Vec::with_capacity(count.get()));
                        ModificationResult::new(true).send_next(values)
                    }
                })
                .unwrap_or(false)
        },
        time_span,
        delay,
    )
}
