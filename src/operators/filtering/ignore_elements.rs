use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable, observable::Subscription, observer::Observer,
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
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for IgnoreElements<OE>
where
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        Filter::new(self.source, |_| false).subscribe(observer)
    }
}
