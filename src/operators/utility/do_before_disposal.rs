//! The [`DoBeforeDisposal`] operator, behind
//! [`ObservableExt::do_before_disposal`](crate::observable::ObservableExt::do_before_disposal).

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

/// Invokes a callback when the subscription is disposed, before the source's subscription is
/// disposed.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::never::Never, utility::do_before_disposal::DoBeforeDisposal},
/// };
///
/// let mut disposed = false;
/// let subscription = DoBeforeDisposal::new(Never, || disposed = true)
///     .subscribe_with_callback(|_| {}, |_| {});
/// drop(subscription);
/// assert!(disposed);
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
    pub fn new(source: OE, callback: F) -> Self
    where
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

delegate_disposal!(
    Disposal<D, F>,
    ChainDisposal<CallbackDisposal<F>, D>,
    where D: Disposable, F: FnOnce()
);

impl<T, E, OE, F> ObservableTypes for DoBeforeDisposal<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal, F>;
}

impl<T, E, OE, F, OR> Observable<OR> for DoBeforeDisposal<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source
            .subscribe(observer)
            .preceded_by(CallbackDisposal::new(self.callback))
            .map_inner_into()
    }
}
