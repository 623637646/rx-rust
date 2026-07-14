use crate::{
    delegate_disposal,
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    safe_lock_option_disposable, safe_lock_option_observer,
    scheduler::{RecursionAction, Scheduler},
    utils::types::{MarkerType, MaybeSend, MutGuard, Mutable, MutableHelper, Shared},
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will observe this Observable.
/// See <https://reactivex.io/documentation/operators/observeon.html>
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
///             utility::observe_on::ObserveOn,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = ObserveOn::new(FromIter::new(vec![1, 2, 3]), handle.clone())
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
///     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ObserveOn<'or, OE, S> {
    source: OE,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> ObserveOn<'or, OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self {
            source,
            scheduler,
            _marker: Default::default(),
        }
    }
}

delegate_disposal!(
    Disposal<T, E, SD, D>,
    crate::disposable::chain_disposal::ChainDisposal<Shared<Mutable<ObserveOnContext<T, E, SD>>>, D>,
    where SD: Disposable, D: Disposable
);

impl<'or, T, E, OE, S> Observable<'static> for ObserveOn<'or, OE, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OE: Observable<'or, T = T, E = E>,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    type T = T;
    type E = E;
    type D = Disposal<T, E, S::D, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let context = Shared::new(Mutable::new(ObserveOnContext {
            values: Vec::new(),
            termination: None,
            disposal: None,
        }));
        let observer = ObserveOnObserver {
            context: context.clone(),
            observer: Shared::new(Mutable::new(Some(observer))),
            scheduler: self.scheduler,
        };
        self.source
            .subscribe(observer)
            .preceded_by(context)
            .map_into()
    }
}

struct ObserveOnContext<T, E, D: Disposable> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
    disposal: Option<Subscription<D>>,
}

// TODO: Disposable should not be Cloneable
impl<T, E, D: Disposable> Disposable for Shared<Mutable<ObserveOnContext<T, E, D>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, disposal);
    }
}

struct ObserveOnObserver<T, E, OR, S: Scheduler> {
    context: Shared<Mutable<ObserveOnContext<T, E, S::D>>>,
    observer: Shared<Mutable<Option<OR>>>,
    scheduler: S,
}

impl<T, E, OR, S: Scheduler> ObserveOnObserver<T, E, OR, S> {
    fn setup_scheduler_if_needed(&self, mut lock: MutGuard<'_, ObserveOnContext<T, E, S::D>>)
    where
        T: MaybeSend + 'static,
        E: MaybeSend + 'static,
        OR: Observer<T, E> + MaybeSend + 'static,
        S: Scheduler + Clone + MaybeSend + 'static,
    {
        if lock.disposal.is_some() {
            return;
        }
        let context = self.context.clone();
        let observer = self.observer.clone();
        // TODO: can remove this recursion? because the values will be sent in a single batch.
        lock.disposal
            .replace(self.scheduler.schedule_recursively(
                move |_| {
                    context.lock_mut(|mut lock| {
                        let termination = lock.termination.take();
                        let values = std::mem::take(&mut lock.values);

                        match (termination, values.is_empty()) {
                            (None, true) => {
                                // No more values. Stop scheduler. Set disposal to None.
                                if let Some(disposal) = lock.disposal.take() {
                                    disposal.dispose();
                                }
                                RecursionAction::Stop
                            }
                            (None, false) => {
                                drop(lock);
                                safe_lock_option_observer!(on_next: observer, values: values);
                                RecursionAction::ContinueImmediately
                            }
                            (Some(termination), true) => {
                                drop(lock);
                                safe_lock_option_observer!(on_termination: observer, termination);
                                RecursionAction::Stop
                            }
                            (Some(termination), false) => {
                                drop(lock);
                                match termination {
                                    Termination::Completed => {
                                        safe_lock_option_observer!(on_next_and_termination: observer, values: values, Termination::Completed);
                                    }
                                    Termination::Error(_) => {
                                        safe_lock_option_observer!(on_termination: observer, termination);
                                    }
                                }
                                RecursionAction::Stop
                            }
                        }
                    })
                },
                None,
            ));
    }
}

impl<T, E, OR, S> Observer<T, E> for ObserveOnObserver<T, E, OR, S>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    OR: Observer<T, E> + MaybeSend + 'static,
    S: Scheduler + Clone + MaybeSend + 'static,
{
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            lock.values.push(value);
            self.setup_scheduler_if_needed(lock);
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.lock_mut(|mut lock| {
            lock.termination.replace(termination);
            self.setup_scheduler_if_needed(lock);
        });
    }
}
