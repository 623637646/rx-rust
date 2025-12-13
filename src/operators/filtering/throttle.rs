use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::safe_lock_option_disposable;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::time::Duration;

/// Emits an item from the source Observable then ignores subsequent items for a particular time span.
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
///         observable::observable_ext::ObservableExt,
///         observer::Termination,
///         operators::{
///             creating::from_iter::FromIter,
///             filtering::throttle::Throttle,
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
///     let subscription = Throttle::new(
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
///     assert_eq!(&*values.lock().unwrap(), &[1]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throttle<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Throttle<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Throttle<OE, S>
where
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let disposal = Shared::new(Mutable::new(None));
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            scheduler: self.scheduler,
            disposal: disposal.clone(),
        }) + disposal
    }
}

struct ThrottleObserver<OR, S> {
    observer: OR,
    time_span: Duration,
    scheduler: S,
    disposal: Shared<Mutable<Option<BoxedDisposal<'static>>>>, // Non-Null means is cooling down.
}

impl<T, E, OR, S> Observer<T, E> for ThrottleObserver<OR, S>
where
    OR: Observer<T, E>,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.disposal.lock_mut(|mut lock| {
            if lock.is_some() {
                return;
            }
            let disposal = self.disposal.clone();
            *lock = Some(BoxedDisposal::new(self.scheduler.schedule(
                move || {
                    assert!(safe_lock_option_disposable!(dispose: disposal));
                },
                Some(self.time_span),
            )));
            drop(lock);
            self.observer.on_next(value);
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.disposal.dispose();
        self.observer.on_termination(termination);
    }
}
