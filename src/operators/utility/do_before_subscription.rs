use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the Observable is subscribed to, before the subscription is established.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_before_subscription::DoBeforeSubscription,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let called = Arc::new(Mutex::new(false));
/// let called_observer = Arc::clone(&called);
///
/// DoBeforeSubscription::new(FromIter::new(vec![1]), move || {
///     *called_observer.lock().unwrap() = true;
/// })
/// .subscribe_with_callback(|_| {}, |_| {});
///
/// assert!(*called.lock().unwrap());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoBeforeSubscription<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeSubscription<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoBeforeSubscription<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(),
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        self.source
            .hook_on_subscription(move |observable, observer| {
                (self.callback)();
                observable.subscribe(observer)
            })
            .subscribe(observer)
    }
}
