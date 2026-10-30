//! The [`DoBeforeSubscription`] operator, behind
//! [`ObservableExt::do_before_subscription`](crate::observable::ObservableExt::do_before_subscription).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback when the observable is subscribed to, before the source is subscribed to.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{
///         creating::just::Just, utility::do_before_subscription::DoBeforeSubscription,
///     },
/// };
/// use std::cell::RefCell;
///
/// // `Just` emits while it is being subscribed to, so its value comes after the callback.
/// let events = RefCell::new(Vec::new());
/// DoBeforeSubscription::new(Just::new(1), || events.borrow_mut().push("callback"))
///     .subscribe_with_callback(|_| events.borrow_mut().push("value"), |_| {});
/// assert_eq!(*events.borrow(), ["callback", "value"]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoBeforeSubscription<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeSubscription<OE, F> {
    /// Creates a [`DoBeforeSubscription`] over `source`;
    /// [`ObservableExt::do_before_subscription`](crate::observable::ObservableExt::do_before_subscription)
    /// is the fluent form.
    pub fn new(source: OE, callback: F) -> Self
    where
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoBeforeSubscription<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for DoBeforeSubscription<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        (self.callback)();
        self.source.subscribe(observer)
    }
}
