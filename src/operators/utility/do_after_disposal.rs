//! The [`DoAfterDisposal`] operator, behind
//! [`ObservableExt::do_after_disposal`](crate::observable::ObservableExt::do_after_disposal).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, callback_disposal::CallbackDisposal, chain_disposal::ChainDisposal,
        dispose_on_drop::DisposeOnDrop,
    },
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the subscription is disposed, after the source's subscription has been
/// disposed.
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
    /// Creates a [`DoAfterDisposal`] over `source`;
    /// [`ObservableExt::do_after_disposal`](crate::observable::ObservableExt::do_after_disposal) is
    /// the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

delegate_disposal!(
    Disposal<D, F>,
    ChainDisposal<D, CallbackDisposal<F>>,
    where D: Disposable, F: FnOnce()
);

impl<T, E, OE, F> ObservableTypes for DoAfterDisposal<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal, F>;
}

impl<T, E, OE, F, OR> Observable<OR> for DoAfterDisposal<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source
            .subscribe(observer)
            .then(CallbackDisposal::new(self.callback))
            .map_inner_into()
    }
}
