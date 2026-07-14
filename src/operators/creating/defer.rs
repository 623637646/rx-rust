use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
};
use educe::Educe;

/// Do not create the Observable until a Observer subscribes, and create a fresh Observable for each Observer.
/// See <https://reactivex.io/documentation/operators/defer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
pub struct Defer<F>(F);

impl<F> Defer<F> {
    pub fn new<OE>(builder: F) -> Self
    where
        F: FnOnce() -> OE,
    {
        Self(builder)
    }
}

impl<'or, T, E, OE, F> Observable<'or> for Defer<F>
where
    F: FnOnce() -> OE,
    OE: Observable<'or, T = T, E = E>,
{
    type T = T;
    type E = E;
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observable = self.0();
        observable.subscribe(observer)
    }
}
