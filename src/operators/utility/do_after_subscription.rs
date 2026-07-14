use crate::utils::types::MaybeSend;
use crate::{observable::Observable, observable::Subscription, observer::Observer};
use educe::Educe;

/// Invokes a callback when the Observable is subscribed to, after the subscription has been established.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_after_subscription::DoAfterSubscription,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let called = Arc::new(Mutex::new(false));
/// let called_observer = Arc::clone(&called);
///
/// DoAfterSubscription::new(FromIter::new(vec![1]), move || {
///     *called_observer.lock().unwrap() = true;
/// })
/// .subscribe_with_callback(|_| {}, |_| {});
///
/// assert!(*called.lock().unwrap());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterSubscription<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterSubscription<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or> for DoAfterSubscription<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T = T, E = E>,
    F: FnOnce(),
{
    type T = T;
    type E = E;
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let subscription = self.source.subscribe(observer);
        (self.callback)();
        subscription
    }
}
