//! The [`DoAfterNext`] operator, behind
//! [`ObservableExt::do_after_next`](crate::observable::ObservableExt::do_after_next).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source observable, after the item has been
/// emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::from_iter::FromIter, utility::do_after_next::DoAfterNext},
/// };
/// use std::cell::RefCell;
///
/// let events = RefCell::new(Vec::new());
/// DoAfterNext::new(FromIter::new(vec![1, 2]), |value| {
///     events.borrow_mut().push(("callback", value))
/// })
/// .subscribe_with_callback(|value| events.borrow_mut().push(("downstream", value)), |_| {});
/// assert_eq!(
///     *events.borrow(),
///     [("downstream", 1), ("callback", 1), ("downstream", 2), ("callback", 2)]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterNext<OE, F> {
    /// Creates a [`DoAfterNext`] over `source`;
    /// [`ObservableExt::do_after_next`](crate::observable::ObservableExt::do_after_next) is the
    /// fluent form.
    pub fn new<T>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T>,
        F: FnMut(T),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoAfterNext<OE, F>
where
    T: Clone,
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for DoAfterNext<OE, F>
where
    OR: Observer<T, E>,
    T: Clone,
    OE: Observable<DoAfterNextObserver<OR, F>, Item = T, Error = E>,
    F: FnMut(T),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(DoAfterNextObserver {
            observer,
            callback: self.callback,
        })
    }
}

#[doc(hidden)]
pub struct DoAfterNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoAfterNextObserver<OR, F>
where
    T: Clone,
    OR: Observer<T, E>,
    F: FnMut(T),
{
    fn on_next(&mut self, value: T) -> Flow {
        // The callback runs after the notification, so it runs even for the value that stopped
        // downstream: the flow is kept and returned once it has.
        let flow = self.observer.on_next(value.clone());
        (self.callback)(value);
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
