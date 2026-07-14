use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable, observable::Subscription, observer::Observer,
    operators::filtering::element_at::ElementAt,
};
use educe::Educe;

/// Emits only the first item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/first.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::first::First,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = First::new(FromIter::new(vec![10, 20, 30]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![10]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct First<OE> {
    source: OE,
}

impl<OE> First<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for First<OE>
where
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
{
    type D = crate::utils::subscribe_with_auto_dispose_on_termination::Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        // Or `self.source.take(1).subscribe(observer)`
        ElementAt::new(self.source, 0).subscribe(observer)
    }
}
