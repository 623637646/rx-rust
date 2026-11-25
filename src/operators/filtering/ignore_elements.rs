//! Ignores every item, and passes on the termination only.
//!
//! The [`IgnoreElements`] operator, behind
//! [`ObservableExt::ignore_elements`](crate::observable::ObservableExt::ignore_elements).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Ignores every item of an observable, passing on only its termination.
/// See <https://reactivex.io/documentation/operators/ignoreelements.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::just::Just,
///         filtering::ignore_elements::IgnoreElements,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = IgnoreElements::new(Just::new(42));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert!(values.is_empty());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct IgnoreElements<OE> {
    source: OE,
}

impl<OE> IgnoreElements<OE> {
    /// Creates an [`IgnoreElements`] over `source`;
    /// [`ObservableExt::ignore_elements`](crate::observable::ObservableExt::ignore_elements) is the
    /// fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for IgnoreElements<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for IgnoreElements<OE>
where
    OR: Observer<T, E>,
    OE: Observable<IgnoreElementsObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(IgnoreElementsObserver { observer })
    }
}

#[doc(hidden)]
pub struct IgnoreElementsObserver<OR> {
    observer: OR,
}

impl<T, E, OR> Observer<T, E> for IgnoreElementsObserver<OR>
where
    OR: Observer<T, E>,
{
    /// Drops the item: the downstream observer never sees one, so it cannot have stopped, and the
    /// termination is still to come.
    fn on_next(&mut self, _value: T) -> Flow {
        Flow::Continue(())
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
