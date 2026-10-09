//! The [`DoAfterSubscription`] operator, behind
//! [`ObservableExt::do_after_subscription`](crate::observable::ObservableExt::do_after_subscription).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the Observable is subscribed to, after the source has been subscribed
/// to.
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
    /// Creates a [`DoAfterSubscription`] over `source`;
    /// [`ObservableExt::do_after_subscription`](crate::observable::ObservableExt::do_after_subscription)
    /// is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoAfterSubscription<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for DoAfterSubscription<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let subscription = self.source.subscribe(observer);
        (self.callback)();
        subscription
    }
}
