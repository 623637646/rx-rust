use crate::utils::types::MaybeSend;
use crate::{
    disposable::{callback_disposal::CallbackDisposal, chain_disposal::ChainDisposal},
    observable::Observable,
    observable::Subscription,
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the subscription is disposed.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_after_disposal::DoAfterDisposal,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let disposed = Arc::new(Mutex::new(false));
/// let disposed_observer = Arc::clone(&disposed);
///
/// let subscription = DoAfterDisposal::new(
///     FromIter::new(vec![1, 2]),
///     move || *disposed_observer.lock().unwrap() = true,
/// )
/// .subscribe_with_callback(
///     |_value| {},
///     |_termination| {},
/// );
///
/// drop(subscription);
///
/// assert!(*disposed.lock().unwrap());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterDisposal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterDisposal<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or> for DoAfterDisposal<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T = T, E = E>,
    F: FnOnce(),
{
    type T = T;
    type E = E;
    type D = ChainDisposal<OE::D, CallbackDisposal<F>>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source
            .subscribe(observer)
            .then(CallbackDisposal::new(self.callback))
    }
}
