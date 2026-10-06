//! The [`Throw`] source.

use crate::thread_mode::Local;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits no items and terminates with an error.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::throw::Throw,
/// };
/// use std::convert::Infallible;
///
/// let mut terminations = Vec::new();
///
/// Throw::new("boom").subscribe_with_callback(
///     |_: Infallible| -> () { panic!("`Throw` should not emit values") },
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(terminations, vec![Termination::Error("boom")]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throw<E>(E);

impl<E> Throw<E> {
    /// Creates a [`Throw`].
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<E> ObservableTypes for Throw<E> {
    type Item = Infallible;
    type Error = E;
    type Mode = Local;
    type Disposal = ();
}

impl<E, OR> Observable<OR> for Throw<E>
where
    OR: Observer<Infallible, E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        observer.on_termination(Termination::Error(self.0));
        Subscription::default()
    }
}
