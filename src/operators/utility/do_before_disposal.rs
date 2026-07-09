use crate::utils::types::MaybeSend;
use crate::{
    disposable::{callback_disposal::CallbackDisposal, chain_disposal::ChainDisposal},
    observable::Observable,
    observable::Subscription,
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the subscription is disposed, before the disposal logic is executed.
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
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for DoBeforeDisposal<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T, E>,
    F: FnOnce(),
{
    type D = ChainDisposal<CallbackDisposal<F>, OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source
            .subscribe(observer)
            .preceded_by(CallbackDisposal::new(self.callback))
    }
}
