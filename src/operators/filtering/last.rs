use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable, observable::Subscription, observer::Observer,
    operators::filtering::take_last::TakeLast,
};
use educe::Educe;

/// Emits only the last item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/last.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::last::Last,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Last::new(FromIter::new(vec![10, 20, 30]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![30]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Last<OE> {
    source: OE,
}

impl<OE> Last<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or> for Last<OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T = T, E = E>,
    OE::D: MaybeSend + 'or,
{
    type T = T;
    type E = E;
    type D = crate::utils::subscribe_unsub_after_termination::Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        TakeLast::new(self.source, 1).subscribe(observer)
    }
}
