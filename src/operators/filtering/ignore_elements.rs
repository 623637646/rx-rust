//! The [`IgnoreElements`] operator, behind
//! [`ObservableExt::ignore_elements`](crate::observable::ObservableExt::ignore_elements).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    operators::filtering::filter::{Filter, FilterObserver},
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
    OE: Observable<FilterObserver<OR, fn(&T) -> bool>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        Filter::new(self.source, (|_| false) as fn(&T) -> bool).subscribe(observer)
    }
}
