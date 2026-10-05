//! The [`First`] operator, behind
//! [`ObservableExt::first`](crate::observable::ObservableExt::first).

use crate::operators::filtering::element_at::ElementAtObserver;
use crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::Observer,
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
    /// Creates a [`First`] over `source`;
    /// [`ObservableExt::first`](crate::observable::ObservableExt::first) is the fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for First<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = crate::utils::subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode, OE::D>;
}

impl<T, E, OE, OR> Observable<OR> for First<OE>
where
    OR: Observer<T, E>,
    OE: Observable<
            ElementAtObserver<
                AutoDisposeOnTerminationObserver<
                    <OE as ObservableTypes>::Mode,
                    OR,
                    <OE as ObservableTypes>::D,
                >,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        // Or `self.source.take(1).subscribe(observer)`
        ElementAt::new(self.source, 0).subscribe(observer)
    }
}
