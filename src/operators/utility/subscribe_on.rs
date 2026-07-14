use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal},
    observable::{Observable, Subscription},
    observer::Observer,
    scheduler::Scheduler,
    utils::types::{MarkerType, MaybeSend},
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will subscribe to this Observable.
/// See <https://reactivex.io/documentation/operators/subscribeon.html>
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
///             utility::subscribe_on::SubscribeOn,
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
///     let subscription = SubscribeOn::new(FromIter::new(vec![1, 2, 3]), handle.clone())
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
pub struct SubscribeOn<'or, OE, S> {
    source: OE,
    scheduler: S,
    _marker: MarkerType<&'or ()>,
}

impl<'or, OE, S> SubscribeOn<'or, OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self {
            source,
            scheduler,
            _marker: Default::default(),
        }
    }
}

delegate_disposal!(
    Disposal<SD, D>,
    ChainDisposal<SD, SharedDisposal<Subscription<D>>>,
    where SD: Disposable, D: Disposable
);

impl<'or, T, E, OE, S> Observable<'static> for SubscribeOn<'or, OE, S>
where
    OE: Observable<'or, T = T, E = E> + MaybeSend + 'static,
    OE::D: MaybeSend + 'static,
    S: Scheduler,
{
    type T = T;
    type E = E;
    type D = Disposal<S::D, OE::D>;

    fn subscribe(
        self,
        observer: impl Observer<T, E> + MaybeSend + 'static,
    ) -> Subscription<Self::D> {
        let shared_sub = SharedDisposal::default();
        let shared_sub_cloned = shared_sub.clone();
        let disposal = self.scheduler.schedule(
            move || shared_sub_cloned.replace(|| self.source.subscribe(observer)),
            None,
        );
        disposal.then(shared_sub).into()
    }
}
