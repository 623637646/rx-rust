use crate::observable::Subscription;
use crate::scheduler::RecursionAction;
use crate::utils::types::{MarkerType, MaybeSend, Mutable, MutableHelper, Shared};
use crate::{delegate_disposal, disposable::Disposable};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{safe_lock_option_disposable, safe_lock_option_observer};
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

delegate_disposal!(
    Disposal<T, SD, D>,
    crate::disposable::chain_disposal::ChainDisposal<Shared<Mutable<DelayContext<T, SD>>>, D>,
    where SD: Disposable, D: Disposable
);

impl<'or, T, E, OE, S> Observable<'static, T, E> for Delay<'or, OE, S>
where
    T: MaybeSend + 'static,
    OE: Observable<'or, T, E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type D = Disposal<T, S::D, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let context = Shared::new(Mutable::new(DelayContext {
            values: VecDeque::new(),
            timer: None,
        }));
        let delay_observer = DelayObserver {
            delay: self.delay,
            scheduler: self.scheduler,
            context: context.clone(),
            observer: Shared::new(Mutable::new(Some(observer))),
        };
        self.source
            .subscribe(delay_observer)
            .preceded_by(context)
            .map_into()
    }
}

struct DelayContext<T, D: Disposable> {
    values: VecDeque<(Instant, Option<T>)>, // None means completed
    timer: Option<Subscription<D>>,
}

// TODO: Disposable should not be Cloneable
impl<T, D: Disposable> Disposable for Shared<Mutable<DelayContext<T, D>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct DelayObserver<T, OR, S: Scheduler> {
    delay: Duration,
    scheduler: S,
    context: Shared<Mutable<DelayContext<T, S::D>>>,
    observer: Shared<Mutable<Option<OR>>>, // None means terminated or disposed
}

impl<T, OR, S: Scheduler> DelayObserver<T, OR, S> {
    fn emit_value_and_setup_timer_if_needed<E>(&self, value: Option<T>)
    where
        T: MaybeSend + 'static,
        OR: Observer<T, E> + MaybeSend + 'static,
        S: Scheduler + Clone + MaybeSend + 'static,
    {
        self.context.lock_mut(|mut lock| {
            lock.values.push_back((Instant::now() + self.delay, value));
            if lock.timer.is_some() {
                return;
            }
            let context = self.context.clone();
            let observer = self.observer.clone();
            lock.timer = Some(self.scheduler.schedule_recursively(
                move |_| {
                    // Get values that should be sent
                    let (values, completed) = context.lock_mut(|mut lock| {
                        let mut values = Vec::new();
                        let mut completed = false;
                        let now = Instant::now();
                        while let Some((instant, _)) = lock.values.front() {
                            if now < *instant {
                                break;
                            }
                            let value = lock.values.pop_front().unwrap().1;
                            if let Some(value) = value {
                                values.push(value);
                            } else {
                                completed = true;
                                break;
                            }
                        }
                        (values, completed)
                    });

                    if completed {
                        safe_lock_option_observer!(on_next_and_termination: observer, values: values, Termination::Completed);
                        RecursionAction::Stop
                    } else {
                        safe_lock_option_observer!(on_next: observer, values: values);
                        context.lock_mut(|mut lock| {
                            if let Some((next_instant, _)) = lock.values.front() {
                                // Continue
                                RecursionAction::ContinueAt(*next_instant)
                            } else {
                                // No more values. Stop timer. Set timer to None.
                                if let Some(timer) = lock.timer.take() {
                                    drop(lock);
                                    timer.dispose();
                                }
                                RecursionAction::Stop
                            }
                        })
                    }
                },
                Some(self.delay),
            ));
        });
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        self.emit_value_and_setup_timer_if_needed(Some(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.emit_value_and_setup_timer_if_needed(None);
            }
            error @ Termination::Error(_) => {
                self.context.dispose();
                safe_lock_option_observer!(on_termination: self.observer, error);
            }
        }
    }
}
