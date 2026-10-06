//! The [`Last`] operator, behind [`ObservableExt::last`](crate::observable::ObservableExt::last).

use crate::operators::filtering::take_last::TakeLastObserver;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    operators::filtering::take_last::TakeLast,
};
use educe::Educe;

/// Emits only the last item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/last.html>
///
/// A source that completes without an item completes it without one, rather than with an error.
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
    /// Creates a [`Last`] over `source`;
    /// [`ObservableExt::last`](crate::observable::ObservableExt::last) is the fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for Last<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Last<OE>
where
    OR: Observer<T, E>,
    OE: Observable<TakeLastObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        TakeLast::new(self.source, 1).subscribe(observer)
    }
}
