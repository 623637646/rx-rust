use crate::utils::types::MaybeSend;
use crate::{observable::Observable, observable::Subscription, observer::Observer};
use educe::Educe;

/// Invokes a callback when the Observable is subscribed to, before the subscription is established.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for DoBeforeSubscription<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T, E>,
    F: FnOnce(),
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        (self.callback)();
        self.source.subscribe(observer)
    }
}
