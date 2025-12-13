use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
};
use educe::Educe;

/// Emits only the first item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/first.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for First<OE>
where
    OE: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        // Or `self.source.take(1).subscribe(observer)`
        self.source.element_at(0).subscribe(observer)
    }
}
