use crate::utils::types::NecessarySendSync;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;

/// Do not create the Observable until a Observer subscribes, and create a fresh Observable for each Observer.
/// See <https://reactivex.io/documentation/operators/defer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::creating::{defer::Defer, just::Just},
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Defer::new(|| Just::new(5));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![5]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Defer<OE, F>(F)
where
    F: FnOnce() -> OE;

impl<OE, F> Defer<OE, F>
where
    F: FnOnce() -> OE,
{
    pub fn new(builder: F) -> Self {
        Self(builder)
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for Defer<OE, F>
where
    F: FnOnce() -> OE,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let observable = self.0();
        observable.subscribe(observer)
    }
}
