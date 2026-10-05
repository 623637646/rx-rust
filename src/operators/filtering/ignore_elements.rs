//! The [`IgnoreElements`] operator, behind
//! [`ObservableExt::ignore_elements`](crate::observable::ObservableExt::ignore_elements).

use crate::operators::filtering::filter::FilterObserver;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    operators::filtering::filter::Filter,
};
use educe::Educe;

/// Suppresses all notifications from an Observable but `on_termination`.
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
/// let mut terminations = Vec::new();
///
/// let observable = IgnoreElements::new(Just::new(42));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert!(values.is_empty());
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct IgnoreElements<OE> {
    source: OE,
}

impl<OE> IgnoreElements<OE> {
    /// Creates an [`IgnoreElements`] over `source`;
    /// [`ObservableExt::ignore_elements`](crate::observable::ObservableExt::ignore_elements) is the fluent form.
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
    type D = OE::D;
}

impl<T, E, OE, OR> Observable<OR> for IgnoreElements<OE>
where
    OR: Observer<T, E>,
    OE: Observable<FilterObserver<OR, fn(&T) -> bool>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        Filter::new(self.source, (|_| false) as fn(&T) -> bool).subscribe(observer)
    }
}
