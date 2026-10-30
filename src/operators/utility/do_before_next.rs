//! The [`DoBeforeNext`] operator, behind
//! [`ObservableExt::do_before_next`](crate::observable::ObservableExt::do_before_next).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source observable, before the item is emitted
/// to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::from_iter::FromIter, utility::do_before_next::DoBeforeNext},
/// };
/// use std::cell::RefCell;
///
/// let events = RefCell::new(Vec::new());
/// DoBeforeNext::new(FromIter::new([1, 2]), |value: &i32| {
///     events.borrow_mut().push(("callback", *value))
/// })
/// .subscribe_with_callback(|value| events.borrow_mut().push(("downstream", value)), |_| {});
/// assert_eq!(
///     *events.borrow(),
///     [("callback", 1), ("downstream", 1), ("callback", 2), ("downstream", 2)]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoBeforeNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeNext<OE, F> {
    /// Creates a [`DoBeforeNext`] over `source`;
    /// [`ObservableExt::do_before_next`](crate::observable::ObservableExt::do_before_next) is the
    /// fluent form.
    pub fn new<T>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T>,
        F: FnMut(&T),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoBeforeNext<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for DoBeforeNext<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DoBeforeNextObserver<OR, F>, Item = T, Error = E>,
    F: FnMut(&T),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(DoBeforeNextObserver {
            observer,
            callback: self.callback,
        })
    }
}

#[doc(hidden)]
pub struct DoBeforeNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoBeforeNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T),
{
    fn on_next(&mut self, value: T) -> Flow {
        (self.callback)(&value);
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
