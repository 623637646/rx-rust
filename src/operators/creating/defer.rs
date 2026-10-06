//! The [`Defer`] source.

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};
use educe::Educe;

/// Creates the Observable only when an Observer subscribes, a fresh one for each subscription.
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
    /// Creates a [`Defer`].
    pub fn new<OE>(builder: F) -> Self
    where
        F: FnOnce() -> OE,
    {
        Self(builder)
    }
}

impl<T, E, OE, F> ObservableTypes for Defer<F>
where
    F: FnOnce() -> OE,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for Defer<F>
where
    OR: Observer<T, E>,
    F: FnOnce() -> OE,
    OE: Observable<OR, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observable = self.0();
        observable.subscribe(observer)
    }
}
