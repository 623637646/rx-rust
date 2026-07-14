use crate::disposable::bound_drop_disposal::BoundDropDisposal;
use crate::disposable::shared_disposal::SharedDisposal;
use crate::observable::Subscription;
use crate::utils::types::{MarkerType, MaybeSend, MutableBool, MutableBoolHelper, Shared};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal},
};
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
///         observable::ObservableExt,
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
pub struct Throttle<'or, OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> Throttle<'or, OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            _marker: Default::default(),
        }
    }
}

delegate_disposal!(
    Disposal<D, S>,
    ChainDisposal<SharedDisposal<BoundDropDisposal<S::D>>, D>,
    where D: Disposable, S: Scheduler
);

impl<'or, T, E, OE, S> Observable<'static> for Throttle<'or, OE, S>
where
    OE: Observable<'or, T = T, E = E>,
    S: Scheduler + MaybeSend + 'or,
{
    type T = T;
    type E = E;
    type D = Disposal<OE::D, S>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let shared_disposal = SharedDisposal::default();
        self.source
            .subscribe(ThrottleObserver {
                observer,
                time_span: self.time_span,
                scheduler: self.scheduler,
                is_cooling: Shared::new(MutableBool::new(false)),
                shared_disposal: shared_disposal.clone(),
            })
            .preceded_by(shared_disposal)
            .map_into()
    }
}

struct ThrottleObserver<OR, S>
where
    S: Scheduler,
{
    observer: OR,
    time_span: Duration,
    scheduler: S,
    is_cooling: Shared<MutableBool>,
    shared_disposal: SharedDisposal<BoundDropDisposal<S::D>>,
}

impl<T, E, OR, S> Observer<T, E> for ThrottleObserver<OR, S>
where
    OR: Observer<T, E>,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        if !self.is_cooling.change_if_not_equal(true) {
            return;
        }
        self.observer.on_next(value);
        let is_cooling_down = self.is_cooling.clone();
        self.shared_disposal.replace(|| {
            self.scheduler.schedule(
                move || {
                    is_cooling_down.write(false);
                },
                Some(self.time_span),
            )
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.shared_disposal.dispose();
        self.observer.on_termination(termination);
    }
}
