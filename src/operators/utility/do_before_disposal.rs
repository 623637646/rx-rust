//! The [`DoBeforeDisposal`] operator, behind
//! [`ObservableExt::do_before_disposal`](crate::observable::ObservableExt::do_before_disposal).

use crate::{
    disposable::{callback_disposal::CallbackDisposal, chain_disposal::ChainDisposal},
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the subscription is disposed, before the source's subscription is
/// disposed.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_before_disposal::DoBeforeDisposal,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let disposed = Arc::new(Mutex::new(false));
/// let disposed_observer = Arc::clone(&disposed);
///
/// let subscription = DoBeforeDisposal::new(
///     FromIter::new(vec![1, 2]),
///     move || *disposed_observer.lock().unwrap() = true,
/// )
/// .subscribe_with_callback(|_| {}, |_| {});
///
/// drop(subscription);
///
/// assert!(*disposed.lock().unwrap());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoBeforeDisposal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeDisposal<OE, F> {
    /// Creates a [`DoBeforeDisposal`] over `source`;
    /// [`ObservableExt::do_before_disposal`](crate::observable::ObservableExt::do_before_disposal)
    /// is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoBeforeDisposal<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = ChainDisposal<CallbackDisposal<F>, OE::D>;
}

impl<T, E, OE, F, OR> Observable<OR> for DoBeforeDisposal<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        self.source
            .subscribe(observer)
            .preceded_by(CallbackDisposal::new(self.callback))
    }
}
